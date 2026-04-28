const fs = require('fs');

const TAXONOMY = [
  'access-control',
  'runtime-safety',
  'resource-usage',
  'storage',
  'observability',
  'maintainability',
  'optimization',
  'benign',
];

const CATEGORY_MAP = {
  'access-control': { severity: 'critical', title: 'Authorization weakness likely detected' },
  'runtime-safety': { severity: 'high', title: 'Runtime safety risk likely detected' },
  'resource-usage': { severity: 'medium', title: 'Unbounded resource usage risk likely detected' },
  storage: { severity: 'high', title: 'Storage namespace risk likely detected' },
  observability: { severity: 'low', title: 'Observability gap likely detected' },
  maintainability: { severity: 'low', title: 'Maintainability risk likely detected' },
  optimization: { severity: 'low', title: 'Optimization opportunity detected' },
};

function tokenize(text) {
  return String(text || '')
    .toLowerCase()
    .split(/[^a-z0-9_!#]+/g)
    .map((token) => token.trim())
    .filter(Boolean)
    .filter((token) => token.length > 1)
    .flatMap((token) => (token.includes('_') ? token.split('_').filter(Boolean) : [token]));
}

function findSourceText(record) {
  if (!record || typeof record !== 'object') return '';
  if (typeof record.source_code === 'string' && record.source_code.trim()) {
    return record.source_code;
  }

  if (record.snapshot && typeof record.snapshot === 'object') {
    const stack = [record.snapshot];
    while (stack.length) {
      const current = stack.pop();
      if (!current || typeof current !== 'object') continue;
      for (const [key, value] of Object.entries(current)) {
        if (typeof value === 'string' && value.trim().length > 80) {
          const tokenScore = ['pub fn', 'require_auth', '#[contract', 'contractimpl', 'storage'].reduce(
            (sum, needle) => sum + (value.includes(needle) ? 1 : 0),
            0,
          );
          if (tokenScore > 0 || key === 'source_code') return value;
        }
        if (value && typeof value === 'object') stack.push(value);
      }
    }
  }

  return '';
}

function normalizeLabel(label) {
  const value = String(label || '').toLowerCase();
  if (!value || value === 'benign' || value === 'none') return 'benign';
  if (value.includes('auth') || value.includes('access') || value.includes('permission') || value.includes('role')) return 'access-control';
  if (value.includes('overflow') || value.includes('underflow') || value.includes('panic') || value.includes('reentrancy') || value.includes('unsafe') || value.includes('runtime')) return 'runtime-safety';
  if (value.includes('loop') || value.includes('gas') || value.includes('dos') || value.includes('resource') || value.includes('budget')) return 'resource-usage';
  if (value.includes('storage') || value.includes('key') || value.includes('collision') || value.includes('state')) return 'storage';
  if (value.includes('event') || value.includes('observability') || value.includes('logging') || value.includes('trace')) return 'observability';
  if (value.includes('maintain') || value.includes('hardcoded') || value.includes('magic') || value.includes('style')) return 'maintainability';
  if (TAXONOMY.includes(value)) return value;
  return 'optimization';
}

function buildLineHints(source, label) {
  const lines = String(source || '').split(/\r?\n/);
  const patterns = {
    'access-control': [/require_auth\s*\(/, /pub\s+fn\s+.*(set|update|mint|burn|transfer|withdraw|admin|upgrade)/i],
    'runtime-safety': [/panic!\s*\(/, /unwrap\s*\(/, /expect\s*\(/, /(\+=|-=|\*=|\+|\-|\*)/],
    'resource-usage': [/\b(loop|while|for)\b/i],
    storage: [/storage\(\)\./, /Symbol::new/, /DataKey/, /storage key/i],
    observability: [/events\(\)\.publish/, /emit/i],
    maintainability: [/"G[A-Z2-7]{20,}"/, /0x[a-fA-F0-9]{32,}/],
    optimization: [/checked_/i, /saturating_/i],
  };

  const list = patterns[label] || [];
  for (let i = 0; i < lines.length; i += 1) {
    const line = lines[i];
    if (list.some((pattern) => pattern.test(line))) {
      return { line: i + 1, evidence: line.trim() };
    }
  }

  if (lines.length > 0) {
    return { line: 1, evidence: lines[0].trim().slice(0, 200) };
  }
  return { line: 1, evidence: '' };
}

function createEmptyStats() {
  return {
    docs: 0,
    tokenTotals: {},
    labelTokenTotals: {},
    labelDocCounts: {},
    vocab: {},
  };
}

function updateCounts(stats, label, tokens) {
  stats.docs += 1;
  stats.labelDocCounts[label] = (stats.labelDocCounts[label] || 0) + 1;
  if (!stats.labelTokenTotals[label]) stats.labelTokenTotals[label] = {};

  for (const token of tokens) {
    stats.vocab[token] = (stats.vocab[token] || 0) + 1;
    stats.tokenTotals[token] = (stats.tokenTotals[token] || 0) + 1;
    stats.labelTokenTotals[label][token] = (stats.labelTokenTotals[label][token] || 0) + 1;
  }
}

function trainModel(samples) {
  const stats = createEmptyStats();
  const byLabel = {};

  for (const sample of samples) {
    const source = findSourceText(sample);
    if (!source || source.trim().length < 24) continue;
    const label = normalizeLabel(sample.label || (sample.issue && sample.issue.category) || (sample.issue && sample.issue.severity) || 'benign');
    const tokens = tokenize(source);
    if (!tokens.length) continue;
    updateCounts(stats, label, tokens);
    byLabel[label] = (byLabel[label] || 0) + 1;
  }

  const labelPriors = {};
  const labelTokenTotals = {};
  const vocabSize = Object.keys(stats.vocab).length || 1;
  const totalDocs = Math.max(stats.docs, 1);

  for (const label of Object.keys(stats.labelDocCounts)) {
    labelPriors[label] = Math.log(stats.labelDocCounts[label] / totalDocs);
    const tokens = stats.labelTokenTotals[label] || {};
    const tokenCount = Object.values(tokens).reduce((sum, count) => sum + count, 0);
    labelTokenTotals[label] = {
      tokenCount,
      tokenCounts: tokens,
    };
  }

  return {
    version: 1,
    trainedAt: new Date().toISOString(),
    vocabSize,
    docs: stats.docs,
    labels: Object.keys(stats.labelDocCounts).sort(),
    labelPriors,
    labelTokenTotals,
    categoryMap: CATEGORY_MAP,
    taxonomy: TAXONOMY,
    byLabel,
  };
}

function scoreTokens(model, tokens) {
  const scores = {};
  const vocabSize = model.vocabSize || 1;
  for (const label of model.labels || []) {
    const prior = model.labelPriors[label] ?? Math.log(1 / Math.max((model.labels || []).length, 1));
    const tokenTotals = model.labelTokenTotals[label] || { tokenCount: 0, tokenCounts: {} };
    const denom = tokenTotals.tokenCount + vocabSize;
    let score = prior;
    for (const token of tokens) {
      const count = tokenTotals.tokenCounts[token] || 0;
      score += Math.log((count + 1) / denom);
    }
    scores[label] = score;
  }

  const maxScore = Math.max(...Object.values(scores));
  const expScores = Object.fromEntries(Object.entries(scores).map(([label, score]) => [label, Math.exp(score - maxScore)]));
  const total = Object.values(expScores).reduce((sum, value) => sum + value, 0) || 1;
  const probabilities = Object.fromEntries(Object.entries(expScores).map(([label, value]) => [label, value / total]));
  return { scores, probabilities };
}

function predictModel(model, source) {
  const tokens = tokenize(source);
  const { probabilities } = scoreTokens(model, tokens);
  const ranked = Object.entries(probabilities)
    .sort((a, b) => b[1] - a[1])
    .map(([label, probability]) => ({ label, probability }));

  const benignProbability = probabilities.benign ?? 0;
  const vulnerabilityProbability = 1 - benignProbability;
  const score = Math.max(0, Math.min(100, Math.round(100 - vulnerabilityProbability * 85)));

  const findings = ranked
    .filter((item) => item.label !== 'benign' && item.probability >= 0.18)
    .slice(0, 4)
    .map((item) => {
      const meta = model.categoryMap[item.label] || { severity: 'low', title: `${item.label} risk likely detected` };
      const evidence = buildLineHints(source, item.label);
      return {
        id: `ml-${item.label}`,
        title: meta.title,
        severity: meta.severity,
        category: item.label,
        line: evidence.line,
        evidence: evidence.evidence,
        explanation: `Model probability for ${item.label} was ${(item.probability * 100).toFixed(1)}%.`,
        recommendation:
          item.label === 'access-control'
            ? 'Add explicit caller authorization checks before privileged state changes.'
            : item.label === 'runtime-safety'
              ? 'Replace panic/unwrap paths with Result-based error handling and checked arithmetic.'
              : item.label === 'storage'
                ? 'Switch to typed storage keys and ensure namespaces are unique.'
                : item.label === 'resource-usage'
                  ? 'Bound loops and input sizes to avoid instruction-budget exhaustion.'
                  : 'Review the suspicious code path and harden it against failure modes.',
        confidence: Number(item.probability.toFixed(4)),
        weight: Number((0.4 + item.probability * 0.8).toFixed(4)),
      };
    });

  const signals = ranked.slice(0, 5).map((item) => ({
    name: item.label,
    value: Math.round(item.probability * 100),
    contribution: Math.round((item.label === 'benign' ? item.probability : -item.probability) * 15),
    explanation: `Posterior probability for ${item.label}.`,
  }));

  return {
    score,
    grade: score >= 90 ? 'A' : score >= 80 ? 'B' : score >= 70 ? 'C' : score >= 60 ? 'D' : 'F',
    model: model.name || 'ml-vuln-detector-v1',
    findings,
    signals,
    summary:
      findings.length === 0
        ? 'No high-confidence vulnerabilities were detected by the ML baseline.'
        : `Top prediction: ${ranked[0]?.label || 'benign'} (${((ranked[0]?.probability || 0) * 100).toFixed(1)}%).`,
    probabilities,
    topLabel: ranked[0]?.label || 'benign',
  };
}

function loadModel(modelPath) {
  return JSON.parse(fs.readFileSync(modelPath, 'utf8'));
}

function saveModel(modelPath, model) {
  fs.writeFileSync(modelPath, JSON.stringify(model, null, 2));
}

module.exports = {
  TAXONOMY,
  CATEGORY_MAP,
  tokenize,
  findSourceText,
  normalizeLabel,
  trainModel,
  predictModel,
  loadModel,
  saveModel,
  buildLineHints,
};
