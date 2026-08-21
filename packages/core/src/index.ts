export const MAX_MANAGED_DEPTH = 5 as const;
export const ACTIVE_FOLDER_MIN = 1 as const;
export const ACTIVE_FOLDER_MAX = 98 as const;
export const ARCHIVE_NUMBER = 99 as const;
export const ARCHIVE_FOLDER_NAME = "99_ARCHIVE" as const;
export const DEFAULT_AUTO_MOVE_CONFIDENCE = 0.9 as const;
export const DEFAULT_RUNNER_UP_GAP = 0.15 as const;

export interface ParsedManagedFolderName {
  valid: boolean;
  number?: number;
  label?: string;
  isArchive: boolean;
  reason?: string;
}

export function parseManagedFolderName(name: string): ParsedManagedFolderName {
  if (name === ARCHIVE_FOLDER_NAME) {
    return { valid: true, number: ARCHIVE_NUMBER, label: "ARCHIVE", isArchive: true };
  }

  const match = /^(\d{2})_(.+)$/.exec(name);
  if (!match) {
    return { valid: false, isArchive: false, reason: "Folder must use NN_Name format." };
  }

  const number = Number(match[1]);
  const label = match[2].trim();

  if (!label) {
    return { valid: false, isArchive: false, reason: "Folder label cannot be empty." };
  }

  if (number === ARCHIVE_NUMBER) {
    return {
      valid: false,
      number,
      label,
      isArchive: false,
      reason: "99 is reserved exclusively for 99_ARCHIVE.",
    };
  }

  if (number < ACTIVE_FOLDER_MIN || number > ACTIVE_FOLDER_MAX) {
    return {
      valid: false,
      number,
      label,
      isArchive: false,
      reason: "Active managed folders must use numbers 01 through 98.",
    };
  }

  return { valid: true, number, label, isArchive: false };
}

export function recommendNextFolderNumber(existingNames: string[]): number | null {
  const used = new Set<number>();
  for (const name of existingNames) {
    const parsed = parseManagedFolderName(name);
    if (parsed.valid && parsed.number && parsed.number <= ACTIVE_FOLDER_MAX) {
      used.add(parsed.number);
    }
  }

  for (let candidate = ACTIVE_FOLDER_MIN; candidate <= ACTIVE_FOLDER_MAX; candidate += 1) {
    if (!used.has(candidate)) return candidate;
  }

  return null;
}

export interface AutoMoveInput {
  destinationExists: boolean;
  destinationDepth: number;
  requiresNewFolder: boolean;
  confidence: number;
  runnerUpConfidence?: number;
}

export interface PolicyDecision {
  allowed: boolean;
  reasons: string[];
}

export function evaluateAutoMove(input: AutoMoveInput): PolicyDecision {
  const reasons: string[] = [];

  if (!input.destinationExists) reasons.push("Destination folder does not exist.");
  if (input.requiresNewFolder) reasons.push("A new folder would be required.");
  if (input.destinationDepth > MAX_MANAGED_DEPTH) reasons.push("Destination exceeds 5 levels.");
  if (input.confidence < DEFAULT_AUTO_MOVE_CONFIDENCE) reasons.push("Classification confidence is below 90%.");

  if (typeof input.runnerUpConfidence === "number") {
    const gap = input.confidence - input.runnerUpConfidence;
    if (gap < DEFAULT_RUNNER_UP_GAP) {
      reasons.push("Top classification is not sufficiently separated from the runner-up.");
    }
  }

  return { allowed: reasons.length === 0, reasons };
}
