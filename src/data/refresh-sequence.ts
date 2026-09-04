/**
 * Hands out a monotonically increasing ticket per key so an async refresh can
 * tell whether a newer request superseded it before it writes state.
 */
export function createRefreshSequencer<K extends string>() {
  const latest = new Map<K, number>();
  return {
    begin(key: K): number {
      const ticket = (latest.get(key) ?? 0) + 1;
      latest.set(key, ticket);
      return ticket;
    },
    isCurrent(key: K, ticket: number): boolean {
      return latest.get(key) === ticket;
    }
  };
}
