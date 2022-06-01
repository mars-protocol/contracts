import { hello } from './app';

test('adds 1 + 2 to equal 3', () => {
  expect(hello('George')).toBe('Hello George!');
});
