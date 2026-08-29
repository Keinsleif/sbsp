// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

import typia, { type IValidation } from 'typia';
import type { GlobalHostSettings } from './types/GlobalHostSettings';
import type { GlobalRemoteSettings } from './types/GlobalRemoteSettings';

export type GlobalSettings = GlobalHostSettings | GlobalRemoteSettings;
type PlainObject = Record<string, unknown>;

export const settingsValidator: (input: unknown) => IValidation<GlobalSettings> = typia.createValidate<GlobalSettings>();

const settingsPartialValidator: (
  input: unknown,
) => IValidation<Partial<GlobalSettings>> = typia.createValidate<Partial<GlobalSettings>>();

const FORBIDDEN_KEYS = new Set(['__proto__', 'constructor', 'prototype']);

const isObject = (obj: unknown): obj is PlainObject =>
  obj != null && typeof obj === 'object' && !Array.isArray(obj);

const isContainer = (v: unknown): v is Record<string, unknown> => {
  return v != null && typeof v === "object";
};

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

function parseErrorPath(path: string): string[] {
  return path
    .replace(/^\$input\.?/, "")
    .replace(/\[\d+\]$/, "")
    .replace(/\[(\d+)\]/g, ".$1")
    .split(".")
    .filter(Boolean);
}

function getDeepValue(source: unknown, tokens: string[]): unknown {
  let current: unknown = source;
  for (const key of tokens) {
    if (!isContainer(current)) return undefined;
    current = (current as Record<string, unknown>)[key];
  }
  return current;
}

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

export function parseOrDefault(text: string, defaultValues: GlobalSettings): GlobalSettings {
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
) => typia.IValidation<GlobalHostSettings | GlobalRemoteSettings> = typia.json.createValidateParse<
  GlobalHostSettings | GlobalRemoteSettings
>();
