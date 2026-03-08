declare module '@/components/Waves' {
  import type { CSSProperties } from 'react'

  interface WavesProps {
    lineColor?: string
    backgroundColor?: string
    waveSpeedX?: number
    waveSpeedY?: number
    waveAmpX?: number
    waveAmpY?: number
    xGap?: number
    yGap?: number
    friction?: number
    tension?: number
    maxCursorMove?: number
    style?: CSSProperties
    className?: string
  }

  const Waves: (props: WavesProps) => JSX.Element
  export default Waves
}
