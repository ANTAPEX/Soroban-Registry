"use client";

import React, { useEffect, useState } from 'react';
import { getAllBreakerStates } from '../lib/resilience';

export default function BreakerDebug(): JSX.Element {
  const [states, setStates] = useState<Record<string, any>>({});

  useEffect(() => {
    setStates(getAllBreakerStates());
    const id = setInterval(() => setStates(getAllBreakerStates()), 1000);
    return () => clearInterval(id);
  }, []);

  return (
    <div style={{ padding: 16 }}>
      <h2>Breaker States (client)</h2>
      <pre style={{ whiteSpace: 'pre-wrap', wordBreak: 'break-word' }}>{JSON.stringify(states, null, 2)}</pre>
    </div>
  );
}
