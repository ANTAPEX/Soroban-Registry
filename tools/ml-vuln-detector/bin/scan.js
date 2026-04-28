#!/usr/bin/env node
const fs = require('fs');
const path = require('path');
const { loadModel, findSourceText, predictModel } = require('../lib/model');

function readInput(inputPath) {
  const text = fs.readFileSync(inputPath, 'utf8');
  if (inputPath.endsWith('.jsonl')) {
    return text.trim().split(/\r?\n/).filter(Boolean).map((line) => JSON.parse(line));
  }
  return [{ source_code: text, label: 'unknown' }];
}

function main() {
  const modelPath = process.argv[2] || path.resolve(process.cwd(), 'model.json');
  const inputPath = process.argv[3];

  if (!inputPath) {
    console.error('Usage: ml-vuln-scan <model.json> <input.jsonl|source.rs>');
    process.exit(2);
  }

  const model = loadModel(modelPath);
  const samples = readInput(inputPath);
  const outputs = samples.map((sample) => {
    const source = findSourceText(sample) || sample.source_code || '';
    const report = predictModel(model, source);
    return {
      ...report,
      input: inputPath,
      sourceLength: source.length,
    };
  });

  console.log(JSON.stringify(outputs.length === 1 ? outputs[0] : outputs, null, 2));
}

if (require.main === module) {
  main();
}
