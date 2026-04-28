#!/bin/bash
# run_performance_test.sh - Benchmarking script for Soroban Registry database indexes

# Load environment variables
if [ -f .env ]; then
    export $(grep -v '^#' .env | xargs)
fi

DB_URL=${DATABASE_URL:-"postgres://postgres:postgres@localhost:5432/soroban_registry"}

echo "======================================================"
echo "   Soroban Registry Performance Benchmark"
echo "======================================================"

# Check if psql is installed
if ! command -v psql &> /dev/null; then
    echo "Error: psql is not installed. Please install PostgreSQL client tools."
    exit 1
fi

# Run the benchmark SQL script
echo "Running benchmarks using EXPLAIN ANALYZE..."
psql "$DB_URL" -f scripts/benchmark_indexes.sql

echo ""
echo "======================================================"
echo "Benchmark Complete."
echo "Review the 'Execution Time' and 'Index Scan' lines in the output."
echo "If you see 'Seq Scan' on large tables, the indexes may not be applied."
echo "======================================================"
