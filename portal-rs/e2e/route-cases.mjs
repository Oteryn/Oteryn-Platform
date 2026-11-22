export function routeKey({ crate, route }) {
  return `route ${crate} ${route}`;
}

export function routeCase(row, expectations) {
  const title = routeKey(row);
  return expectations.filter(expected => routeKey(expected) === title).length > 1
    ? `${title} [${row.method}]`
    : title;
}
