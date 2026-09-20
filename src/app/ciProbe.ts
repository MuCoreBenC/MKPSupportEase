/* 故意的 CI 失败探针（Task 16.2）。
   下面那个未使用的局部变量会让 eslint 与 tsc 同时报错 —— 用来验证"CI 红了就合不进去"。
   验证完这个文件会被删掉。 */
export function ciProbe() {
  const unused = 1
  return 2
}
