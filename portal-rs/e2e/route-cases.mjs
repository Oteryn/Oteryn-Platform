export function routeCaseTitle(row, expectations) {
  const title = `route ${row.crate} ${row.route}`;
  const hasMultipleActors = expectations.some(other =>
    other.crate === row.crate && other.route === row.route && other.actor !== row.actor);
  const hasMultipleMethods = expectations.some(other =>
    other.crate === row.crate && other.route === row.route
    && other.actor === row.actor && other.method !== row.method);
  const actorTitle = hasMultipleActors ? `${title} (${row.actor})` : title;
  return hasMultipleMethods && row.method !== 'GET' ? `${actorTitle} ${row.method}` : actorTitle;
}

export function routesPassed(cases, projects, titles) {
  return projects.every(project => titles.every(title => {
    const matches = cases.filter(test =>
      test.project === project && test.case === title && test.spec.endsWith('routes.spec.mjs'));
    return matches.length === 1 && matches[0].status === 'passed' && matches[0].attempts === 1;
  }));
}
