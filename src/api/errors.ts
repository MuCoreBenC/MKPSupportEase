/**
 * 「这个口子还没接」的那个错。
 *
 * 故意不做成"界面上一条友好提示"——撤掉 mock 之后，没接的接口应该一眼看得见：
 * 构造时就往控制台打一条 error，然后照常作为异常抛出去，让调用栈把现场留下来。
 * 谁吞掉它、谁在哪个按钮上没接后端，看控制台就知道。
 */
export class NotImplementedError extends Error {
  constructor(method: string) {
    super(`[API] 未实现的接口: ${method}`)
    this.name = 'NotImplementedError'
    console.error(this.message)
  }
}
