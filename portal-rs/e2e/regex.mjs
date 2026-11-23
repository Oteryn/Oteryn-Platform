// RegExp.escape is unavailable on the Node 22 runner. Escape literal values
// before inserting them into a regular expression.
export const escapeRegex = value => String(value).replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
