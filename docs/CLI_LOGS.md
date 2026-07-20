## CLI Logs

The CLI writes lightweight NDJSON logs to:

`~/.soroban-registry/registry-cli.log.ndjson`

### View logs

```bash
soroban-registry log
```

### Tail/follow logs

```bash
soroban-registry log --tail
```

### Filter by level

```bash
soroban-registry log --level error
```

### Filter by service

```bash
soroban-registry log --service cli
```

### Search

```bash
soroban-registry log --search "contract_verified"
```

### Export

```bash
soroban-registry log --export logs.txt
```

