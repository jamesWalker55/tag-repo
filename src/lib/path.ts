import path from 'path';

export class UnsupportedPathError extends Error {
  constructor(message: string) {
    super(message);
    this.name = 'UnsupportedPathError';
  }
}

export function filename(x: string) {
  if (x.includes(':')) throw new UnsupportedPathError('Only relative paths are supported');
  if (x.startsWith('//') || x.startsWith('\\\\')) throw new UnsupportedPathError('Mounted paths are not supported');

  return x.match(/.*[\/\\]/);

  // return path.basename(x);
}
