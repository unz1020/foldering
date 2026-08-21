export type SchemaLevelNumber = 1 | 2 | 3 | 4 | 5;

export interface SchemaLevel { level: SchemaLevelNumber; key: string; name: string; description: string; examples: string[]; }
export interface FolderSchema { id: string; name: string; version: number; levels: SchemaLevel[]; }
export interface SchemaValidationResult { valid: boolean; errors: string[]; }

export function validateSchema(schema: FolderSchema): SchemaValidationResult {
  const errors: string[] = [];
  if (!schema.name.trim()) errors.push("Schema name is required.");
  if (schema.levels.length < 1 || schema.levels.length > 5) errors.push("A schema must contain between 1 and 5 levels.");
  const seenKeys = new Set<string>();
  schema.levels.forEach((item, index) => {
    const expectedLevel = index + 1;
    if (item.level !== expectedLevel) errors.push(`Level ${item.level} is out of sequence; expected ${expectedLevel}.`);
    if (!item.key.trim()) errors.push(`Level ${expectedLevel} needs a stable key.`);
    if (seenKeys.has(item.key)) errors.push(`Duplicate schema key: ${item.key}`);
    seenKeys.add(item.key);
    if (!item.name.trim()) errors.push(`Level ${expectedLevel} needs a display name.`);
    if (!item.description.trim()) errors.push(`Level ${expectedLevel} needs a description.`);
  });
  return { valid: errors.length === 0, errors };
}

export function resequenceLevels(levels: SchemaLevel[]): SchemaLevel[] {
  return levels.map((item, index) => ({ ...item, level: (index + 1) as SchemaLevelNumber }));
}

export function createLevel(level: SchemaLevelNumber): SchemaLevel {
  return { level, key: `level_${level}`, name: `Level ${level}`, description: "이 단계가 무엇을 의미하는지 설명하세요.", examples: [] };
}
