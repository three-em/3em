export const getTagSource = `
function getTag(tx, field) {
  const encodedName = btoa(field);
  return atob(tx.tags.find((data) => data.name === encodedName)?.value || "");
}`;
