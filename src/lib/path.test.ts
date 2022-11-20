import {describe, expect, test} from 'vitest';
import {filename, UnsupportedPathError} from './path';

describe('filename', function() {
  // Windows
  test('01', () => expect(filename('c:\\foo\\bar')).toBe('bar'));
  test('01', () => expect(filename('c:\\foo\\bar\\')).toBe('bar'));
  test('02', () => expect(() => filename('\\\\conky\\mountpoint\\foo\\bar')).toThrowError(UnsupportedPathError));
  test('03', () => expect(() => filename('c:\\')).toThrowError(UnsupportedPathError));
  test('04', () => expect(() => filename('\\\\conky\\mountpoint\\')).toThrowError(UnsupportedPathError));
  test('05', () => expect(() => filename('c:/')).toThrowError(UnsupportedPathError));
  // Unix
  test('06', () => expect(() => filename('//conky/mountpoint/')).toThrowError(UnsupportedPathError));
  test('07', () => expect(filename('/foo/bar')).toBe('bar'));
  test('08', () => expect(() => filename('/')).toThrowError(UnsupportedPathError));
  test('09', () => expect(filename('foo')).toBe('foo'));
  test('10', () => expect(() => filename('////foo')).toThrowError(UnsupportedPathError));
  test('11', () => expect(() => filename('//foo//bar')).toThrowError(UnsupportedPathError));
});

// describe('dirname', function() {
//   // Windows
//   test('01', () => expect(dirname('c:\\foo\\bar')).toBe('c:\\foo'));
//   test('02', () => expect(dirname('\\\\conky\\mountpoint\\foo\\bar')).toBe('\\\\conky\\mountpoint\\foo'));
//   test('03', () => expect(dirname('c:\\')).toBe('c:\\'));
//   test('04', () => expect(dirname('\\\\conky\\mountpoint\\')).toBe('\\\\conky\\mountpoint\\'));
//   // Unix
//   test('05', () => expect(dirname('c:/')).toBe('c:/'));
//   test('06', () => expect(dirname('//conky/mountpoint/')).toBe('//conky/mountpoint/'));
//   test('07', () => expect(dirname('/foo/bar')).toBe('/foo'));
//   test('08', () => expect(dirname('/')).toBe('/'));
//   test('09', () => expect(dirname('foo')).toBe(''));
//   test('10', () => expect(dirname('////foo')).toBe('////'));
//   test('11', () => expect(dirname('//foo//bar')).toBe('//foo'));
// });
