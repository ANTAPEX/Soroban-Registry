#!/usr/bin/env node
const fs = require('fs');
const path = require('path');
const { trainModel, saveModel } = require('../lib/model');

function main() {
  const input = process.argv[2] || path.resolve(process.cwd(), 'dataset.jsonl');
  const output = process.argv[3] || path.resolve(process.cwd(), 'model.json');

  const raw = fs.readFileSync(input, 'utf8').trim();
  const samples = raw
    ? raw
        .split(/\r?\n/)
        .filter(Boolean)
        .map((line) => JSON.parse(line))
    : [];

  const model = trainModel(samples);
  model.name = 'ml-vuln-detector-v1';
  saveModel(output, model);
  console.log(JSON.stringify({
    trainedAt: model.trainedAt,
    docs: model.docs,
    labels: model.labels,
    vocabSize: model.vocabSize,
    output,
  }, null, 2));
}

if (require.main === module) {
  main();
}
