
export type AppError = 
  | { Message: string}
  | { NotImplemented: string};

export function parseAppError(error: unknown): AppError | null {
  try {
    if (typeof error === 'object' && error !== null ) {
      return error as AppError;
    } else if (typeof error === 'string' && error !== null) {
      return JSON.parse(error) as AppError;
    } else {
      return null;
    }
  } catch (error) {
    return null;
  }
}

export function displayAppErrorMessage(error: AppError): string {
  if ('Message' in error) {
    return error.Message;
  } else if ('NotImplemented' in error) {
    return `Not Implemented: ${error.NotImplemented}`;
  } else {
    return 'Unknown error';
  }
}