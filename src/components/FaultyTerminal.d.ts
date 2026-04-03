import type { CSSProperties, HTMLAttributes } from 'react'

declare interface FaultyTerminalProps extends HTMLAttributes<HTMLDivElement> {
  scale?: number
  gridMul?: [number, number]
  digitSize?: number
  timeScale?: number
  pause?: boolean
  scanlineIntensity?: number
  glitchAmount?: number
  flickerAmount?: number
  noiseAmp?: number
  chromaticAberration?: number
  dither?: number | boolean
  curvature?: number
  tint?: string
  backgroundTint?: string
  mouseReact?: boolean
  mouseStrength?: number
  dpr?: number
  pageLoadAnimation?: boolean
  brightness?: number
  className?: string
  style?: CSSProperties
}

declare const FaultyTerminal: (props: FaultyTerminalProps) => JSX.Element

export default FaultyTerminal
