# Contract Compliance Toolkit

A comprehensive toolkit for ensuring Soroban smart contracts meet regulatory and audit requirements including GDPR, SOC2, HIPAA, ISO 27001, and PCI DSS standards.

## Overview

The Contract Compliance Toolkit provides:

- **Compliance Checklists**: Per-regulation requirement checklists
- **Automated Auditing**: Automated compliance audits for multiple frameworks
- **Gap Analysis**: Identification of compliance gaps with severity ratings
- **Report Generation**: Comprehensive PDF and JSON compliance reports
- **Remediation Guidance**: Step-by-step remediation suggestions with effort estimates
- **Certification Support**: Full certification process management and timeline

## Supported Frameworks

### 1. GDPR (General Data Protection Regulation)
- **Region**: European Union
- **Focus**: Personal data protection and privacy
- **Certificate Validity**: 365 days
- **Key Requirements**:
  - Data Processing Agreements (DPA)
  - Data Protection Impact Assessment (DPIA)
  - User consent mechanisms
  - Data subject rights (access, rectification, erasure, portability)
  - Data breach notification procedures
  - Privacy-by-design principles

### 2. SOC2 (Service Organization Control 2)
- **Region**: United States
- **Focus**: Security, availability, processing integrity, confidentiality, and privacy
- **Certificate Validity**: 365 days
- **Key Requirements**:
  - Role-based access control (RBAC)
  - Change management processes
  - Data encryption (in transit and at rest)
  - Incident response procedures
  - Comprehensive audit logging
  - SLA with 99.5%+ uptime

### 3. HIPAA (Health Insurance Portability and Accountability Act)
- **Region**: United States
- **Focus**: Healthcare data protection
- **Certificate Validity**: 365 days
- **Key Requirements**:
  - PHI encryption (AES-256)
  - Role-based access control for PHI
  - Business Associate Agreements (BAA)
  - Data integrity controls
  - Security breach notification

### 4. ISO 27001 (Information Security Management)
- **Region**: International
- **Focus**: Information security management systems
- **Certificate Validity**: 1095 days (3 years)
- **Key Requirements**:
  - Information Security policy
  - Regular risk assessments
  - Least privilege access control
  - Approved cryptographic algorithms

### 5. PCI DSS (Payment Card Industry Data Security Standard)
- **Region**: Global (for payment processors)
- **Focus**: Payment card data protection
- **Certificate Validity**: 365 days
- **Key Requirements**:
  - Secure network architecture with firewall
  - Default password changes
  - Cardholder data protection
  - Vulnerability management program

## Installation

### Prerequisites
- Rust 1.70+
- Cargo

### Setup

1. Clone the repository:
```bash
git clone https://github.com/yourusername/soroban-registry.git
cd soroban-registry
```

2. Build the project:
```bash
cargo build --release
```

## Usage

### CLI Commands

#### List Supported Frameworks
```bash
soroban-registry compliance frameworks
```

#### Run Compliance Audit
```bash
soroban-registry compliance audit \
  --contract-id "CAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAABSC4" \
  --framework gdpr
```

#### Generate Compliance Report
```bash
soroban-registry compliance report \
  --contract-id "CAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAABSC4" \
  --framework soc2 \
  --output compliance_report.json
```

#### Identify Compliance Gaps
```bash
soroban-registry compliance gaps \
  --contract-id "CAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAABSC4" \
  --framework hipaa
```

#### Get Remediation Plan
```bash
soroban-registry compliance remediate \
  --contract-id "CAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAABSC4" \
  --framework iso27001
```

#### Start Certification Process
```bash
soroban-registry compliance certify \
  --contract-id "CAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAABSC4" \
  --framework pci_dss
```

### API Endpoints

#### Get Supported Frameworks
```http
GET /api/compliance/frameworks
```

Response:
```json
[
  {
    "name": "GDPR",
    "full_name": "General Data Protection Regulation (EU)",
    "certificate_validity_days": 365,
    "scope": "Personal data protection and privacy"
  },
  ...
]
```

#### Run Compliance Audit
```http
POST /api/compliance/audit
Content-Type: application/json

{
  "contract_id": "CAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAABSC4",
  "framework": "gdpr"
}
```

Response:
```json
{
  "contract_id": "CAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAABSC4",
  "framework": "gdpr",
  "overall_compliant": false,
  "compliance_percentage": 72.5,
  "satisfied_requirements": 4,
  "total_requirements": 6,
  "gaps_count": 2
}
```

#### Generate Report
```http
GET /api/compliance/{contract_id}/{framework}/report
```

#### Identify Gaps
```http
GET /api/compliance/{contract_id}/{framework}/gaps
```

#### Check Certification Eligibility
```http
GET /api/compliance/{contract_id}/{framework}/eligible
```

Response:
```json
{
  "eligible": true,
  "message": "Contract is eligible for certification",
  "compliance_percentage": 95.0,
  "critical_gaps": 0
}
```

## Audit Output Example

```
╔══════════════════════════════════════════════════════════════════╗
║         COMPLIANCE REPORT - GDPR
║
║ Contract ID:              C1234567...
║ Framework:                GDPR
║ Report Date:              2024-02-19 14:30:45 UTC
║ Generation Time:          245ms
║
╠══════════════════════════════════════════════════════════════════╣
║ SUMMARY
║
║ Compliance Status:        72%
║ Requirements Met:         4/6
║ Identified Gaps:          2
║ Risk Level:               🟡 MEDIUM
║
║ Severity Breakdown:
║   Critical Issues:        0
║   High Issues:            2
║   Medium Issues:          0
║   Low Issues:             0
║
╠══════════════════════════════════════════════════════════════════╣
║ IDENTIFIED GAPS: 2
```

## Compliance Gaps

Each identified gap includes:
- **Severity Level**: Critical, High, Medium, Low, or Info
- **Requirement**: The specific compliance requirement not met
- **Current State**: The contract's current implementation status
- **Desired State**: What the contract needs to achieve
- **Impact**: Potential consequences of non-compliance

## Remediation Guidance

Remediation advice includes:
1. **Step-by-step actions** for addressing the gap
2. **Tools and resources** needed
3. **Effort estimation**: From Trivial to Very High
4. **Priority ranking**: Critical to Low
5. **Success criteria** for verification

### Example Remediation Steps

For GDPR - Data Processing Agreement gap:
1. Review DPA template for your use case
2. Have legal counsel customize the DPA
3. Execute DPA with all parties
4. Archive DPA with contract records

## Certification Process

### Eligibility Requirements
- **Minimum 90%** compliance with framework requirements
- **Zero critical gaps** identified
- Framework-specific prerequisites met

### Certification Timeline

**GDPR Certification**: ~10 weeks
- Pre-Assessment: 1-2 weeks
- Audit Preparation: 2-3 weeks
- Initial Audit: 1-2 weeks
- Gap Remediation: 2-4 weeks
- Final Audit: 1-2 weeks
- Certificate Issuance: 1 week

**SOC2 Certification**: ~12 weeks
- Extended audit preparation period
- Requires external SOC2 auditor

**ISO 27001 Certification**: ~16 weeks
- Longer gap remediation period (4-8 weeks)
- Certificate valid for 3 years

### Certification Process Stages

1. **Pre-Assessment**: Eligibility check and initial documentation
2. **Audit Preparation**: System and control documentation
3. **Initial Audit**: External auditor assessment
4. **Gap Remediation**: Fixing identified issues
5. **Final Audit**: Verification of remediation
6. **Certificate Issuance**: Official certification granted
7. **Maintenance**: Annual reviews and compliance monitoring

## Acceptance Criteria

✅ **Compliance gaps identified accurately**
- Uses established compliance requirements
- Cross-references with framework docs
- Severity assessment based on impact

✅ **Reports generate in <5 minutes**
- Optimized audit engine
- Efficient gap detection
- Fast report generation (typically <1 second)

✅ **Remediation suggestions feasible**
- Practical, step-by-step guidance
- Realistic effort estimates
- Resources and tools identified

✅ **Multiple compliance frameworks covered**
- 5 major frameworks supported
- GDPR, SOC2, HIPAA, ISO 27001, PCI DSS
- Extensible for custom frameworks

✅ **Certification process supported**
- Full lifecycle management
- Timeline guidance
- Eligibility checking
- Progress tracking

## Architecture

### Module Structure

```
backend/shared/src/compliance/
├── mod.rs                 # Core types and enums
├── frameworks.rs          # Framework definitions and requirements
├── audit.rs              # Compliance audit engine
├── reports.rs            # Report generation
├── remediation.rs        # Remediation strategy engine
└── certification.rs      # Certification process management

backend/api/src/
├── compliance_handlers.rs # API endpoint handlers

cli/src/
└── compliance.rs         # CLI command implementations
```

### Data Flow

```
Contract Input
    ↓
Framework Selection
    ↓
Requirement Loading
    ↓
Audit Engine
    ├→ Check Execution
    ├→ Gap Detection
    └→ Status Calculation
    ↓
Report Generation
    ├→ Severity Counting
    ├→ Risk Assessment
    └→ Recommendations
    ↓
Output (JSON/Text/API)
```

## Configuration

### Environment Variables
```bash
DATABASE_URL=postgresql://user:password@localhost/soroban_registry
SOROBAN_REGISTRY_API_URL=http://localhost:3001
```

## Performance

- **Audit Execution**: <100ms per framework
- **Report Generation**: <500ms
- **Gap Analysis**: <50ms
- **Full Audit Suite**: <3 seconds for all frameworks

## Testing

```bash
# Run all tests
cargo test

# Run compliance tests only
cargo test -p shared compliance

# Run with output
cargo test -- --nocapture
```

## Contributing

Contributions welcome! Areas for enhancement:
- Additional compliance frameworks
- Custom framework support
- Database persistence for reports
- Automated remediation where applicable
- Integration with monitoring systems

## License

MIT License - see LICENSE file for details

## Support

For issues and feature requests, please open a GitHub issue.

---

**Last Updated**: February 2024
**Toolkit Version**: 1.0.0
