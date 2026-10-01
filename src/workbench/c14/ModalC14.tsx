/*
 * 模态框的统一入口（C14 原样移植）：把外壳交出的遮罩宿主（`.shellBody`）
 * 塞给共用件的 `Modal`（它的 `host` 参，见 overlayHost.ts）。
 *
 * 为什么包一层而不是各处自己 `useOverlayHost()`：工作台里有二十几处 `<Modal>`，
 * 散着传 host 每处都要写一遍、漏一处那个框就退回「盖住半屏」的老样子；
 * 收在这一个文件里，调用方只换一个名字，行为没有第二种。
 *
 * 外壳还没挂上时宿主是 null —— `Modal` 那边原地渲染，不画没宿主的浮层。
 */

import { Modal } from '../components/modal'
import type { ModalProps } from '../components/modal'
import { useOverlayHost } from './overlayHost'

export default function ModalC14(props: ModalProps) {
  const host = useOverlayHost()
  return <Modal {...props} host={host} />
}
