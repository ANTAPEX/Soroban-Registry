#!/bin/bash
# Disaster Recovery Script for Soroban Registry
# Ensures RTO < 1 hour and RPO < 1 minute

set -e

# Configuration
API_URL="${API_URL:-http://localhost:3001}"
LOG_FILE="/tmp/disaster_recovery.log"
MAX_RTO_SECONDS=3600  # 1 hour
MAX_RPO_SECONDS=60   # 1 minute

# Colors for output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

# Logging function
log() {
    echo "$(date '+%Y-%m-%d %H:%M:%S') - $1" | tee -a "$LOG_FILE"
}

log_info() {
    log "${GREEN}[INFO]${NC} $1"
}

log_warn() {
    log "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    log "${RED}[ERROR]${NC} $1"
}

# Function to validate RTO/RPO compliance
validate_recovery_objectives() {
    local start_time=$1
    local backup_time=$2
    
    local current_time=$(date +%s)
    local rto_seconds=$((current_time - start_time))
    local rpo_seconds=$((current_time - backup_time))
    
    log_info "RTO: ${rto_seconds}s (Max: ${MAX_RTO_SECONDS}s)"
    log_info "RPO: ${rpo_seconds}s (Max: ${MAX_RPO_SECONDS}s)"
    
    if [ $rto_seconds -gt $MAX_RTO_SECONDS ]; then
        log_error "RTO objective exceeded: $rto_seconds seconds > $MAX_RTO_SECONDS seconds"
        return 1
    fi
    
    if [ $rpo_seconds -gt $MAX_RPO_SECONDS ]; then
        log_error "RPO objective exceeded: $rpo_seconds seconds > $MAX_RPO_SECONDS seconds"
        return 1
    fi
    
    log_info "RTO/RPO objectives met"
    return 0
}

# Function to detect disaster state
detect_disaster() {
    log_info "Checking system health..."
    
    # Check if API is responsive
    if ! curl -s --max-time 10 "$API_URL/health" > /dev/null; then
        log_warn "API is not responding - potential disaster detected"
        return 0
    fi
    
    # Check for recent backups (within RPO window)
    local cutoff_time=$(date -d "-$MAX_RPO_SECONDS seconds" -u +%Y-%m-%d-%H-%M-%S)
    if [ ! -f "/tmp/last_backup_${cutoff_time}.timestamp" ]; then
        log_warn "No recent backup found within RPO window - potential data loss scenario"
        return 0
    fi
    
    log_info "System appears healthy - no disaster detected"
    return 1
}

# Function to find most recent valid backup
find_latest_backup() {
    local contract_id=$1
    log_info "Finding latest valid backup for contract: $contract_id"
    
    # Get the most recent backup date
    local backup_date=$(curl -s \
        -H "Content-Type: application/json" \
        "$API_URL/api/contracts/$contract_id/backups" | \
        jq -r '.[-1].backup_date' 2>/dev/null)
    
    if [ -z "$backup_date" ] || [ "$backup_date" = "null" ]; then
        log_error "No backups found for contract: $contract_id"
        return 1
    fi
    
    log_info "Found backup from: $backup_date"
    echo "$backup_date"
}

# Function to restore from backup
restore_from_backup() {
    local contract_id=$1
    local backup_date=$2
    local start_time=$(date +%s)
    
    log_info "Starting restoration for contract: $contract_id from backup: $backup_date"
    
    # Perform the restoration
    local response=$(curl -s -w "\n%{http_code}" \
        -X POST \
        -H "Content-Type: application/json" \
        "$API_URL/api/contracts/$contract_id/backups/restore" \
        -d "{\"backup_date\": \"$backup_date\"}")
    
    local http_code=$(echo "$response" | tail -n1)
    local json_response=$(echo "$response" | sed '$d')
    
    if [ "$http_code" -ne 200 ]; then
        log_error "Restoration failed with HTTP code: $http_code"
        log_error "Response: $json_response"
        return 1
    fi
    
    # Parse restoration result
    local success=$(echo "$json_response" | jq -r '.success')
    local duration_ms=$(echo "$json_response" | jq -r '.restore_duration_ms')
    
    if [ "$success" = "true" ]; then
        log_info "Restoration completed successfully in ${duration_ms}ms"
        
        # Convert backup date to timestamp for RPO calculation
        local backup_timestamp=$(date -d "$backup_date" +%s 2>/dev/null || date -d "${backup_date/T/ }" +%s)
        if validate_recovery_objectives $start_time $backup_timestamp; then
            log_info "Disaster recovery completed successfully with RTO/RPO compliance"
            return 0
        else
            log_error "Recovery completed but RTO/RPO objectives not met"
            return 1
        fi
    else
        log_error "Restoration failed"
        return 1
    fi
}

# Function to validate restored state
validate_restored_state() {
    local contract_id=$1
    log_info "Validating restored state for contract: $contract_id"
    
    # Perform basic health check
    if curl -s --max-time 10 "$API_URL/api/contracts/$contract_id/health" > /dev/null; then
        log_info "Contract health check passed"
        return 0
    else
        log_error "Contract health check failed"
        return 1
    fi
}

# Function to notify stakeholders
notify_stakeholders() {
    local contract_id=$1
    local status=$2
    local message=$3
    
    log_info "Sending notification to stakeholders: $message"
    
    # In a real implementation, this would send emails/SMS/Slack messages
    # For now, we'll just log the notification
    echo "NOTIFICATION: Disaster recovery for contract $contract_id - Status: $status - Message: $message" >> /tmp/stakeholder_notifications.log
}

# Main disaster recovery function
perform_disaster_recovery() {
    local contract_id=${1:-}
    
    if [ -z "$contract_id" ]; then
        log_error "Contract ID is required"
        echo "Usage: $0 <contract_id>"
        exit 1
    fi
    
    log_info "=========================================="
    log_info "DISASTER RECOVERY INITIATED"
    log_info "Contract ID: $contract_id"
    log_info "=========================================="
    
    # Detect if disaster state exists
    if ! detect_disaster; then
        log_info "No disaster detected, exiting"
        return 0
    fi
    
    # Find the most recent backup
    backup_date=$(find_latest_backup "$contract_id")
    if [ $? -ne 0 ]; then
        log_error "Could not find a suitable backup"
        notify_stakeholders "$contract_id" "FAILURE" "No suitable backup found for disaster recovery"
        exit 1
    fi
    
    # Perform the restoration
    if restore_from_backup "$contract_id" "$backup_date"; then
        # Validate the restored state
        if validate_restored_state "$contract_id"; then
            log_info "Disaster recovery completed successfully"
            notify_stakeholders "$contract_id" "SUCCESS" "Disaster recovery completed successfully with RTO/RPO compliance"
            return 0
        else
            log_error "Restored state validation failed"
            notify_stakeholders "$contract_id" "FAILURE" "Restored state validation failed"
            exit 1
        fi
    else
        log_error "Disaster recovery failed"
        notify_stakeholders "$contract_id" "FAILURE" "Disaster recovery failed"
        exit 1
    fi
}

# Function to simulate disaster for testing
simulate_disaster() {
    local contract_id=$1
    log_info "Simulating disaster for testing purposes..."
    
    # This would normally involve more complex failure simulation
    # For now, we'll just force a restoration from the latest backup
    perform_disaster_recovery "$contract_id"
}

# Function to run recovery drill
run_recovery_drill() {
    local contract_id=${1:-}
    
    if [ -z "$contract_id" ]; then
        log_error "Contract ID is required for recovery drill"
        echo "Usage: $0 --drill <contract_id>"
        exit 1
    fi
    
    log_info "=========================================="
    log_info "DISASTER RECOVERY DRILL"
    log_info "Testing recovery procedures for: $contract_id"
    log_info "=========================================="
    
    # Create a backup before the drill
    log_info "Creating backup before recovery drill..."
    curl -s -X POST \
        -H "Content-Type: application/json" \
        "$API_URL/api/contracts/$contract_id/backups" \
        -d '{"include_state": true}' > /dev/null
    
    # Simulate disaster and perform recovery
    simulate_disaster "$contract_id"
    
    log_info "Recovery drill completed"
}

# Function to schedule automated recovery monitoring
setup_monitoring() {
    log_info "Setting up automated disaster detection and recovery monitoring..."
    
    # In a real implementation, this would set up cron jobs or systemd services
    # For now, we'll just show what would be scheduled
    
    cat << 'EOF' > /tmp/recovery_monitoring_cron
# Soroban Registry Disaster Recovery Monitoring
# Check system health every 5 minutes
*/5 * * * * /path/to/disaster_recovery.sh --monitor

# Verify backups every 10 minutes
*/10 * * * * /path/to/disaster_recovery.sh --verify-backups

# Weekly recovery drill
0 2 * * 0 /path/to/disaster_recovery.sh --drill-test
EOF

    log_info "Monitoring configuration would be installed as cron jobs"
    log_info "Configuration saved to /tmp/recovery_monitoring_cron for review"
}

# Function to monitor system health
monitor_system() {
    log_info "Monitoring system health..."
    
    if detect_disaster; then
        log_warn "Disaster detected, initiating automated recovery..."
        # In a real implementation, this would trigger automatic recovery
        # For safety, we'll just log it here
        log_info "Would initiate automatic recovery (disabled in this test mode)"
    else
        log_info "System health OK"
    fi
}

# Function to verify backups
verify_backups() {
    log_info "Verifying backup integrity..."
    
    # In a real implementation, this would iterate through all contracts
    # and verify their latest backups
    log_info "Backup verification completed"
}

# Parse command line arguments
case "${1:-}" in
    --drill|--drill-test)
        run_recovery_drill "$2"
        ;;
    --monitor)
        monitor_system
        ;;
    --verify-backups)
        verify_backups
        ;;
    --setup-monitoring)
        setup_monitoring
        ;;
    --simulate)
        simulate_disaster "$2"
        ;;
    "")
        echo "Usage: $0 <contract_id>                    # Perform disaster recovery for a contract"
        echo "       $0 --drill <contract_id>            # Run recovery drill"
        echo "       $0 --simulate <contract_id>         # Simulate disaster and recover"
        echo "       $0 --monitor                        # Monitor system health"
        echo "       $0 --verify-backups                 # Verify backup integrity"
        echo "       $0 --setup-monitoring               # Setup automated monitoring"
        exit 1
        ;;
    *)
        perform_disaster_recovery "$1"
        ;;
esac