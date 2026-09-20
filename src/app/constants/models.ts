import type { Option } from './machines'

/** 机型。id 同时是 heroArt 里整机图的键（见 src/app/heroArt.ts） */
export const models: Option[] = [
  { id: 'a1', label: 'A1' },
  { id: 'a1mini', label: 'A1 mini' },
  { id: 'a2l', label: 'A2L' },
  { id: 'p1s', label: 'P1S' },
  { id: 'p2s', label: 'P2S' },
  { id: 'x1c', label: 'X1C' },
]
