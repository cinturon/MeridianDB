
export type AppError =
  | { Message: string }
  | { NotImplemented: string };

function tryParseAppErrorJson(value: string): AppError | null {
  try {
    const parsed = JSON.parse(value) as unknown;
    if (typeof parsed === "object" && parsed !== null) {
      if ("Message" in parsed || "NotImplemented" in parsed) {
        return parsed as AppError;
      }
    }
  } catch {
    // Not JSON — fall through.
  }
  return null;
}

export function parseAppError(error: unknown): AppError | null {
  if (typeof error === "string") {
    return tryParseAppErrorJson(error) ?? { Message: error };
  }

  if (error instanceof Error) {
    return tryParseAppErrorJson(error.message) ?? { Message: error.message };
  }

  if (typeof error === "object" && error !== null) {
    const record = error as Record<string, unknown>;
    if ("Message" in record || "NotImplemented" in record) {
      return error as AppError;
    }
    if (typeof record.message === "string") {
      return (
        tryParseAppErrorJson(record.message) ?? { Message: record.message }
      );
    }
  }

  return null;
}

export function displayAppErrorMessage(error: AppError): string {
  if ("Message" in error) {
    return error.Message;
  } else if ("NotImplemented" in error) {
    return `Not Implemented: ${error.NotImplemented}`;
  } else {
    return "Unknown error";
  }
}
