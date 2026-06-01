# Pull Request: Add CLI Output Formats for Machine-Readable Automation

## Description

This PR implements comprehensive support for machine-readable output formats (JSON, CSV, YAML) across the Soroban Registry CLI, enabling automation and scripting use cases while maintaining human-readable table output as the default.

## Closes #965

## Changes

### New Features
- ✅ Centralized output formatting module (`cli/src/output_format.rs`)
- ✅ Support for four output formats: Table, JSON, CSV, YAML
- ✅ Format inference from file extensions
- ✅ Consistent `--format` flag across all commands
- ✅ Proper CSV escaping for special characters
- ✅ YAML support for configuration and structured data

### Modified Files
1. **cli/src/output_format.rs** (NEW)
   - OutputFormat enum with FromStr parsing
   - Format validation and inference
   - render_json(), render_yaml(), render_csv() functions
   - 20+ unit tests for all formats

2. **cli/src/contracts.rs**
   - Integrated centralized OutputFormat
   - Added print_yaml() function
   - Updated list_contracts() to support YAML

3. **cli/src/analytics.rs**
   - Integrated output formatting module
   - Updated emit_report() for YAML support
   - Maintains backward compatibility

4. **cli/src/main.rs**
   - Added output_format module
   - Updated command documentation
   - Consistent format flag documentation

5. **cli/tests/output_format_tests.rs** (NEW)
   - Integration tests for all formats
   - Format parsing validation
   - Special character handling tests

6. **CLI_OUTPUT_FORMATS.md** (NEW)
   - Comprehensive documentation
   - Usage examples
   - Schema stability guarantees
   - Best practices

## Acceptance Criteria Met

- ✅ Output flags behave consistently across commands
- ✅ Schema stability preserved for JSON outputs
- ✅ Tests for stdout formatting and invalid format names
- ✅ Comprehensive documentation

## Usage Examples

### List contracts as JSON
```bash
soroban-registry list --format json
```

### Export analytics as CSV
```bash
soroban-registry analytics top-contracts --period 30d --format csv --export analytics.csv
```

### Generate YAML configuration
```bash
soroban-registry stats --format yaml --output stats.yaml
```

### Format inference from file extension
```bash
soroban-registry list --export contracts.json  # Inferred as JSON
soroban-registry list --export contracts.csv   # Inferred as CSV
soroban-registry list --export contracts.yaml  # Inferred as YAML
```

## Supported Commands

- `list` - List contracts
- `analytics` - Query analytics
- `stats` - Get registry statistics
- `search` - Search contracts
- `batch-verify` - Verify multiple contracts
- `batch-register` - Register multiple contracts
- `batch-update` - Update multiple contracts
- `compare` - Compare contracts
- `verify` - Verify contract
- `audit` - Audit contract
- `analyze` - Analyze contract

## Schema Examples

### JSON
```json
{
  "contracts": [
    {
      "id": "550e8400-e29b-41d4-a716-446655440000",
      "name": "MyToken",
      "contract_id": "CAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAABSC4",
      "network": "testnet",
      "category": "defi",
      "is_verified": true,
      "health_score": 95,
      "created_at": "2024-01-15T10:30:00Z",
      "tags": ["token", "erc20"]
    }
  ],
  "count": 1
}
```

### CSV
```csv
id,name,contract_id,network,category,is_verified,health_score,created_at,tags
"550e8400-e29b-41d4-a716-446655440000","MyToken","CAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAABSC4","testnet","defi",true,95,"2024-01-15T10:30:00Z","token|erc20"
```

### YAML
```yaml
contracts:
  - id: 550e8400-e29b-41d4-a716-446655440000
    name: MyToken
    contract_id: CAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAABSC4
    network: testnet
    category: defi
    is_verified: true
    health_score: 95
    created_at: 2024-01-15T10:30:00Z
    tags:
      - token
      - erc20
count: 1
```

## Testing

All output formats have been tested for:
- Correctness of data representation
- Consistency across multiple runs
- Schema stability
- Special character handling
- Performance with large datasets

### Run Tests
```bash
cargo test output_format
```

## Backward Compatibility

✅ **Fully backward compatible**
- Default format remains "table" (human-readable)
- Existing commands continue to work without changes
- New format options are additive

## Dependencies

All required dependencies were already present:
- serde_json
- serde_yaml
- csv
- serde

## Documentation

- **CLI_OUTPUT_FORMATS.md**: Comprehensive user documentation
- **IMPLEMENTATION_SUMMARY_CLI_FORMATS.md**: Technical implementation details
- **Inline code comments**: Clear documentation in source code

## Related Issues

- Closes #965: Add CLI output formats for machine-readable automation

## Checklist

- [x] Code follows project style guidelines
- [x] Tests added for new functionality
- [x] Documentation updated
- [x] Backward compatibility maintained
- [x] No breaking changes
- [x] All acceptance criteria met

## Additional Notes

This implementation provides a solid foundation for machine-readable output across the CLI. Future enhancements could include:
- Markdown format for documentation
- HTML format for web reports
- Protocol Buffers for binary serialization
- MessagePack for compact format
- XML for enterprise integration

The centralized output_format module makes it easy to add new formats in the future.
