export const pathQuote = (filePath: string) => {
  if (filePath.includes("\"") || !filePath.includes(" ")) {
    return filePath
  }

  return `"${filePath}"`
}
