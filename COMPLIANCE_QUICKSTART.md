# Compliance Toolkit Quick Start Guide

## 5-Minute Setup

### 1. Check Available Frameworks
```bash
soroban-registry compliance frameworks
```

### 2. Run Your First Audit
```bash
soroban-registry compliance audit \
  --contract-id "CAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAABSC4" \
  --framework gdpr
```

Expected output:
```
Compliance Audit
==================================================================
Contract ID: CAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAABSC4
Framework: GDPR

Audit Results:
  Overall Status: ⚠️ NON-COMPLIANT
  Compliance Score: 72.5%
  Requirements Met: 4/6
  Identified Gaps: 2
  Last Checked: 2024-02-19 14:30:45 UTC

Identified Gaps:
   1. Data Processing Agreement [🔴 CRITICAL]
      A Data Processing Agreement (DPA) is required by GDPR.
      Current: No documentation or tags indicating compliance measures
   ...
```

### 3. Get Remediation Plan
```bash
soroban-registry compliance remediate \
  --contract-id "CAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAABSC4" \
  --framework gdpr
```

### 4. Generate Report
```bash
soroban-registry compliance report \
  --contract-id "CAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAABSC4" \
  --framework gdpr \
  --output report.json
```

### 5. Check Certification Eligibility
```bash
soroban-registry compliance certify \
  --contract-id "CAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAABSC4" \
  --framework soc2
```

## Common Workflows

### Audit Multiple Frameworks
```bash
for framework in gdpr soc2 hipaa iso27001 pci_dss; do
  soroban-registry compliance audit \
    --contract-id "YOUR_CONTRACT_ID" \
    --framework $framework
done
```

### Generate Comprehensive Compliance Bundle
```bash
CONTRACT_ID="YOUR_CONTRACT_ID"
OUTPUT_DIR="compliance_reports"
mkdir -p $OUTPUT_DIR

for framework in gdpr soc2 hipaa; do
  soroban-registry compliance report \
    --contract-id $CONTRACT_ID \
    --framework $framework \
    --output "$OUTPUT_DIR/${framework}_report.json"
done
```

### Export Results
```bash
# JSON export
soroban-registry compliance report \
  --contract-id "CONTRACT_ID" \
  --framework gdpr \
  --output compliance_report.json

# View as text
soroban-registry compliance audit \
  --contract-id "CONTRACT_ID" \
  --framework gdpr
```

## Using the API

### Python Example
```python
import requests
import json

API_URL = "http://localhost:3001"
CONTRACT_ID = "CAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAABSC4"

# Get frameworks
response = requests.get(f"{API_URL}/api/compliance/frameworks")
frameworks = response.json()

# Run audit
audit_request = {
    "contract_id": CONTRACT_ID,
    "framework": "gdpr"
}
response = requests.post(f"{API_URL}/api/compliance/audit", json=audit_request)
audit_result = response.json()

# Generate report
response = requests.get(f"{API_URL}/api/compliance/{CONTRACT_ID}/gdpr/report")
report = response.json()

# Check gaps
response = requests.get(f"{API_URL}/api/compliance/{CONTRACT_ID}/gdpr/gaps")
gaps = response.json()

print(json.dumps(audit_result, indent=2))
```

### JavaScript/Node.js Example
```javascript
const API_URL = "http://localhost:3001";
const CONTRACT_ID = "CAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAABSC4";

// Get frameworks
const frameworks = await fetch(`${API_URL}/api/compliance/frameworks`)
  .then(r => r.json());

// Run audit
const auditResult = await fetch(`${API_URL}/api/compliance/audit`, {
  method: "POST",
  headers: { "Content-Type": "application/json" },
  body: JSON.stringify({
    contract_id: CONTRACT_ID,
    framework: "gdpr"
  })
}).then(r => r.json());

// Generate report
const report = await fetch(
  `${API_URL}/api/compliance/${CONTRACT_ID}/gdpr/report`
).then(r => r.json());

console.log(auditResult);
```

### cURL Examples
```bash
# Get frameworks
curl http://localhost:3001/api/compliance/frameworks

# Run audit
curl -X POST http://localhost:3001/api/compliance/audit \
  -H "Content-Type: application/json" \
  -d '{
    "contract_id": "CAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAABSC4",
    "framework": "gdpr"
  }'

# Get report
curl http://localhost:3001/api/compliance/CONTRACT_ID/gdpr/report

# Identify gaps
curl http://localhost:3001/api/compliance/CONTRACT_ID/gdpr/gaps

# Check eligibility
curl http://localhost:3001/api/compliance/CONTRACT_ID/gdpr/eligible
```

## Interpreting Results

### Compliance Score
- **90-100%**: Likely eligible for certification
- **75-89%**: Minor gaps needing remediation
- **50-74%**: Significant work required
- **<50%**: Major compliance overhaul needed

### Severity Levels
- 🔴 **CRITICAL**: Must fix before certification
- 🟠 **HIGH**: Significant compliance risk
- 🟡 **MEDIUM**: Should address soon
- 🟢 **LOW**: Nice-to-have improvements
- ℹ️ **INFO**: Informational only

### Effort Estimates
- **Trivial**: < 1 hour
- **Low**: 1-2 hours
- **Medium**: 2-8 hours
- **High**: 1-3 days
- **Very High**: 3+ days

## Remediation Priorities

1. **Fix Critical Issues First**
   - Address all 🔴 critical gaps immediately
   - These prevent certification

2. **Address High Priority Gaps**
   - 🟠 high severity issues
   - Significant compliance risks

3. **Medium and Low Priority**
   - Can be scheduled after critical issues
   - Plan larger implementation phases

## Certification Path

### For GDPR
1. Audit contract: `compliance audit`
2. Identify gaps: `compliance gaps`
3. Review remediation: `compliance remediate`
4. Check eligibility: `compliance certify`
5. Schedule DPA and DPIA review
6. Update contract documentation
7. Complete certification process

### For SOC2
1. Run audit
2. Plan change management process
3. Implement access controls and logging
4. Schedule external SOC2 auditor
5. Complete audit
6. Receive certification

### For HIPAA
1. Audit contract
2. Classify PHI in contract
3. Implement encryption
4. Execute BAAs with participants
5. Test incident response
6. Pass compliance audit

## Troubleshooting

### "Contract not found"
Ensure the contract ID is valid and properly formatted.

### "Unsupported compliance framework"
Use one of: gdpr, soc2, hipaa, iso27001, pci_dss

### "API connection failed"
Check that the API server is running:
```bash
curl http://localhost:3001/health
```

### Reports taking too long
The toolkit is optimized to generate reports in <5 seconds. If slower:
1. Check API server performance
2. Verify database connection
3. Check system resources

## Next Steps

1. **Review Your Framework Requirements**
   - Understand which regulations apply to your contract
   - Read the full framework specifications

2. **Run Initial Audit**
   - Get baseline compliance status
   - Identify all gaps

3. **Prioritize Remediation**
   - Address critical issues first
   - Plan implementation timeline

4. **Start Certification Process**
   - Once eligible, initiate certification
   - Work with external auditors where required

5. **Maintain Compliance**
   - Schedule regular audits (quarterly recommended)
   - Keep documentation current
   - Plan for recertification

---

**Support**: For help, check [COMPLIANCE_TOOLKIT.md](./COMPLIANCE_TOOLKIT.md) or open an issue.
