
'use client';

import { useState } from 'react';
import * as StellarSdk from '@stellar/stellar-sdk';
import CodeCopyButton from './CodeCopyButton';
import { useCopy } from '@/hooks/useCopy';

interface CodeRunnerProps {
  initialCode: string;
  language: 'javascript' | 'rust';
  copyAnalytics?: {
    contractId?: string;
    exampleId?: string;
    exampleTitle?: string;
  };
}

export default function CodeRunner({
  initialCode,
  language,
  copyAnalytics,
}: CodeRunnerProps) {
  const [code, setCode] = useState(initialCode);
  const [output, setOutput] = useState<string>('');
  const [isRunning, setIsRunning] = useState(false);
  // Shared copy hook handles clipboard state and analytics event emission.
  const { copy, copied, isCopying } = useCopy();

  const handleCopyCode = async () => {
    await copy(code, {
      successEventName: 'contract_code_copied',
      failureEventName: 'contract_code_copy_failed',
      analyticsParams: {
        // Context passed from parent so copied events are tied to contract/example.
        ...copyAnalytics,
        language,
      },
    });
  };

  const runCode = async () => {
    if (language !== 'javascript') {
      setOutput('Running Rust code in the browser is not supported yet.');
      return;
    }

    setIsRunning(true);
    setOutput('');

    try {
      // Capture console.log
      const logs: string[] = [];
      const originalLog = console.log;
      const originalError = console.error;

      console.log = (...args) => {
        logs.push(args.map(a => String(a)).join(' '));
        originalLog(...args);
      };

      console.error = (...args) => {
        logs.push(`ERROR: ${args.map(a => String(a)).join(' ')}`);
        originalError(...args);
      };

      // Create a secure context for execution
      // Note: This is a basic implementation. For production, consider using a sandboxed iframe or worker.
      // We expose StellarSdk to the code.
      const func = new Function('StellarSdk', 'console', `
        return (async () => {
          try {
            ${code}
          } catch (e) {
            console.error(e);
          }
        })();
      `);

      await func(StellarSdk, console);

      // Restore console
      console.log = originalLog;
      console.error = originalError;

      setOutput(logs.join('\n') || 'Code executed successfully (no output).');
    } catch (err: unknown) {
      const message = err instanceof Error ? err.message : 'Unknown error';
      setOutput(`Execution Error: ${message}`);
    } finally {
      setIsRunning(false);
    }
  };

  return (
    <div className="flex flex-col gap-4">
      <div className="relative rounded-lg overflow-hidden border border-gray-200 dark:border-gray-800">
        <div className="bg-gray-100 dark:bg-gray-800 px-4 py-2 flex items-center justify-between border-b border-gray-200 dark:border-gray-700">
          <span className="text-xs font-mono text-gray-500 uppercase">{language}</span>
          <div className="flex items-center gap-2">
            <CodeCopyButton onCopy={handleCopyCode} copied={copied} disabled={isCopying} />
            {language === 'javascript' && (
              <button
                onClick={runCode}
                disabled={isRunning}
                className={`px-3 py-1 rounded-md text-xs font-medium text-white transition-colors ${
                  isRunning ? 'bg-gray-400 cursor-not-allowed' : 'bg-green-600 hover:bg-green-700'
                }`}
              >
                {isRunning ? 'Running...' : 'Run Code'}
              </button>
            )}
          </div>
        </div>
        <textarea
          value={code}
          onChange={(e) => setCode(e.target.value)}
          className="w-full h-64 p-4 font-mono text-sm bg-gray-50 dark:bg-gray-950 text-gray-900 dark:text-gray-100 focus:outline-none resize-none"
          spellCheck={false}
        />
      </div>

      {(output || isRunning) && (
        <div className="rounded-lg bg-gray-900 text-gray-100 p-4 font-mono text-sm overflow-x-auto">
          <div className="text-gray-500 text-xs mb-2 uppercase">Output</div>
          <pre className="whitespace-pre-wrap">{output}</pre>
        </div>
      )}
    </div>
  );
}
