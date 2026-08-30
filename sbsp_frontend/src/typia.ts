// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

import typia, { type IValidation } from 'typia';
import type { GlobalRemoteSettings } from './types/GlobalRemoteSettings';

type PlainObject = Record<string, unknown>;

export const settingsValidator: (input: unknown) => IValidation<GlobalRemoteSettings> = typia.createValidate<GlobalRemoteSettings>();

const settingsPartialValidator: (
  input: unknown,
) => IValidation<Partial<GlobalRemoteSettings>> = typia.createValidate<Partial<GlobalRemoteSettings>>();

const FORBIDDEN_KEYS = new Set(['__proto__', 'constructor', 'prototype']);

const isObject = (obj: unknown): obj is PlainObject =>
  obj != null && typeof obj === 'object' && !Array.isArray(obj);

const isContainer = (v: unknown): v is Record<string, unknown> => {
  return v != null && typeof v === "object";
};

/**
 * Deeply merges properties from a source object into a target object.
 *
 * Nested plain objects are merged recursively, while other source values replace
 * corresponding target values. Prototype-related keys are ignored.
 *
 * @param target - The object providing the initial properties
 * @param source - The object whose properties are merged into the target
 * @returns A merged copy of the target object
 */
function mergeDeeply<T extends PlainObject>(target: T, source: PlainObject): T {
  if (!isObject(target) || !isObject(source)) {
    return { ...target };
  }

  const result: PlainObject = { ...target };

  for (const [sourceKey, sourceValue] of Object.entries(source)) {

    if (sourceKey === '__proto__' || sourceKey === 'constructor' || sourceKey === 'prototype') {
      continue;
    }

    const targetValue = target[sourceKey];

    if (isObject(sourceValue) && isObject(targetValue) && Object.prototype.hasOwnProperty.call(target, sourceKey)) {
      result[sourceKey] = mergeDeeply(targetValue, sourceValue);
    } else {
      result[sourceKey] = sourceValue;
    }
  }

  return result as T;
}

/**
 * Converts a validation error path into an array of property tokens.
 *
 * @param path - The validation error path to parse
 * @returns The property and array-index tokens extracted from the path
 */
function parseErrorPath(path: string): string[] {
  return path
    .replace(/^\$input\.?/, "")
    .replace(/\[\d+\]$/, "")
    .replace(/\[(\d+)\]/g, ".$1")
    .split(".")
    .filter(Boolean);
}

/**
 * Retrieves a nested value from a source object using property tokens.
 *
 * @param source - The value to traverse
 * @param tokens - The property names and array indices defining the nested path
 * @returns The value at the specified path, or `undefined` if traversal reaches a non-container
 */
function getDeepValue(source: unknown, tokens: string[]): unknown {
  let current: unknown = source;
  for (const key of tokens) {
    if (!isContainer(current)) return undefined;
    current = (current as Record<string, unknown>)[key];
  }
  return current;
}

/**
 * Assigns a value at a nested path, creating intermediate objects or arrays as needed.
 *
 * @param target - The object to update
 * @param tokens - Property names and array indices forming the nested path
 * @param value - The value to assign
 */
function setDeepValue(target: Record<string, unknown>, tokens: string[], value: unknown): void {
  let current: Record<string, unknown> = target;
  for (let i = 0; i < tokens.length - 1; i++) {
    const key = tokens[i];
    if (key == null || FORBIDDEN_KEYS.has(key)) return;
    if (!isContainer(current[key])) {
      const nextKey = tokens[i + 1];
      if (nextKey == null) return;
      current[key] = /^\d+$/.test(nextKey) ? [] : {};
    }
    current = current[key] as Record<string, unknown>;
  }
  const last = tokens[tokens.length - 1];
  if (last) current[last] = value;
}

/**
 * Parses remote settings and merges them with default values.
 *
 * Invalid JSON or non-object input produces a cloned copy of the defaults. Invalid
 * setting values are replaced with their corresponding defaults.
 *
 * @param text - The JSON-encoded settings
 * @param defaultValues - The settings used for missing or invalid values
 * @returns The merged and validated remote settings
 */
export function parseOrDefault(text: string, defaultValues: GlobalRemoteSettings): GlobalRemoteSettings {
  let parsed: unknown;
  try {
    parsed = JSON.parse(text);
  } catch {
    return structuredClone(defaultValues);
  }
  if (!isObject(parsed)) {
    return structuredClone(defaultValues);
  }

  const inputObject = parsed as Record<string, unknown>;

  const result = settingsPartialValidator(inputObject);
  const resultObject = mergeDeeply(structuredClone(defaultValues), inputObject);

  if (!result.success) {
    for (const error of result.errors) {
      const keys = parseErrorPath(error.path);
      if (keys.length > 0) {
        const defaultValue = getDeepValue(defaultValues, keys);
        if (defaultValue !== undefined) {
          setDeepValue(resultObject, keys, defaultValue);
        }
      }
    }
  }

  return resultObject;
}

export const settingsParser: (
  input: string,
) => typia.IValidation<GlobalRemoteSettings> = typia.json.createValidateParse<
  GlobalRemoteSettings
>();
