/**
 * Formal-verification types for Soroban Registry
 */

export interface FormalVerificationSession {
  verifier_version: string;
  created_at: string;
}

export interface FormalVerificationPropertyResult {
  id: string;
  status: "Proved" | "Violated" | "Unknown";
  message?: string;
  counterexample?: string | null;
}

export interface FormalVerificationPropertyDefinition {
  property_id: string;
  description?: string;
  invariant?: string;
}

export interface FormalVerificationProperty {
  property: FormalVerificationPropertyDefinition;
  result: FormalVerificationPropertyResult;
}

export interface FormalVerificationFinding {
  id: string;
  title: string;
  description: string;
  severity: string;
  category: string;
  cwe_id?: string | null;
  affected_functions: string[];
  remediation: string;
}

export interface ProofCertificate {
  properties_proved: number;
  properties_violated: number;
  properties_inconclusive: number;
  overall_confidence: number;
  summary: string;
  generated_at: string;
}

export interface FormalVerificationReport {
  session: FormalVerificationSession;
  properties: FormalVerificationProperty[];
  vulnerabilities: FormalVerificationFinding[];
  certificate?: ProofCertificate | null;
}
