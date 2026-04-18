import { Color, Mesh, Program, Renderer, Triangle } from 'ogl'
import { useCallback, useEffect, useMemo, useRef, useState } from 'react'
import './FaultyTerminal.css'

const RGB_MATCHER = /[\d.]+/g

const vertexShader = `
attribute vec2 position;
attribute vec2 uv;
varying vec2 vUv;
void main() {
  vUv = uv;
  gl_Position = vec4(position, 0.0, 1.0);
}
`

const fragmentShader = `
precision mediump float;

varying vec2 vUv;

uniform float iTime;
uniform vec3  iResolution;
uniform float uScale;

uniform vec2  uGridMul;
uniform float uDigitSize;
uniform float uScanlineIntensity;
uniform float uGlitchAmount;
uniform float uFlickerAmount;
uniform float uNoiseAmp;
uniform float uChromaticAberration;
uniform float uDither;
uniform float uCurvature;
uniform vec3  uTint;
uniform vec3  uBackground;
uniform vec2  uMouse;
uniform float uMouseStrength;
uniform float uUseMouse;
uniform float uPageLoadProgress;
uniform float uUsePageLoadAnimation;
uniform float uBrightness;

float time;

float hash21(vec2 p){
  p = fract(p * 234.56);
  p += dot(p, p + 34.56);
  return fract(p.x * p.y);
}

float noise(vec2 p)
{
  return sin(p.x * 10.0) * sin(p.y * (3.0 + sin(time * 0.090909))) + 0.2;
}

mat2 rotate(float angle)
{
  float c = cos(angle);
  float s = sin(angle);
  return mat2(c, -s, s, c);
}

float fbm(vec2 p)
{
  p *= 1.1;
  float f = 0.0;
  float amp = 0.5 * uNoiseAmp;

  mat2 modify0 = rotate(time * 0.02);
  f += amp * noise(p);
  p = modify0 * p * 2.0;
  amp *= 0.454545;

  mat2 modify1 = rotate(time * 0.02);
  f += amp * noise(p);
  p = modify1 * p * 2.0;
  amp *= 0.454545;

  mat2 modify2 = rotate(time * 0.08);
  f += amp * noise(p);

  return f;
}

float pattern(vec2 p, out vec2 q, out vec2 r) {
  vec2 offset1 = vec2(1.0);
  vec2 offset0 = vec2(0.0);
  mat2 rot01 = rotate(0.1 * time);
  mat2 rot1 = rotate(0.1);

  q = vec2(fbm(p + offset1), fbm(rot01 * p + offset1));
  r = vec2(fbm(rot1 * q + offset0), fbm(q + offset0));
  return fbm(p + r);
}

float digit(vec2 p){
    vec2 grid = uGridMul * 15.0;
    vec2 s = floor(p * grid) / grid;
    p = p * grid;
    vec2 q, r;
    float intensity = pattern(s * 0.1, q, r) * 1.3 - 0.03;

    if(uUseMouse > 0.5){
        vec2 mouseWorld = uMouse * uScale;
        float distToMouse = distance(s, mouseWorld);
        float mouseInfluence = exp(-distToMouse * 8.0) * uMouseStrength * 10.0;
        intensity += mouseInfluence;

        float ripple = sin(distToMouse * 20.0 - iTime * 5.0) * 0.1 * mouseInfluence;
        intensity += ripple;
    }

    if(uUsePageLoadAnimation > 0.5){
        float cellRandom = fract(sin(dot(s, vec2(12.9898, 78.233))) * 43758.5453);
        float cellDelay = cellRandom * 0.8;
        float cellProgress = clamp((uPageLoadProgress - cellDelay) / 0.2, 0.0, 1.0);

        float fadeAlpha = smoothstep(0.0, 1.0, cellProgress);
        intensity *= fadeAlpha;
    }

    p = fract(p);
    p *= uDigitSize;

    float px5 = p.x * 5.0;
    float py5 = (1.0 - p.y) * 5.0;
    float x = fract(px5);
    float y = fract(py5);

    float i = floor(py5) - 2.0;
    float j = floor(px5) - 2.0;
    float n = i * i + j * j;
    float f = n * 0.0625;

    float isOn = step(0.1, intensity - f);
    float brightness = isOn * (0.2 + y * 0.8) * (0.75 + x * 0.25);

    return step(0.0, p.x) * step(p.x, 1.0) * step(0.0, p.y) * step(p.y, 1.0) * brightness;
}

float onOff(float a, float b, float c)
{
  return step(c, sin(iTime + a * cos(iTime * b))) * uFlickerAmount;
}

float displace(vec2 look)
{
    float y = look.y - mod(iTime * 0.25, 1.0);
    float window = 1.0 / (1.0 + 50.0 * y * y);
    return sin(look.y * 20.0 + iTime) * 0.0125 * onOff(4.0, 2.0, 0.8) * (1.0 + cos(iTime * 60.0)) * window;
}

vec3 getColor(vec2 p){

    float bar = step(mod(p.y + time * 20.0, 1.0), 0.2) * 0.4 + 1.0;
    bar *= uScanlineIntensity;

    float displacement = displace(p);
    p.x += displacement;

    if (uGlitchAmount != 1.0) {
      float extra = displacement * (uGlitchAmount - 1.0);
      p.x += extra;
    }

    float middle = digit(p);

    const float off = 0.002;
    float sum = digit(p + vec2(-off, -off)) + digit(p + vec2(0.0, -off)) + digit(p + vec2(off, -off)) +
                digit(p + vec2(-off, 0.0)) + digit(p + vec2(0.0, 0.0)) + digit(p + vec2(off, 0.0)) +
                digit(p + vec2(-off, off)) + digit(p + vec2(0.0, off)) + digit(p + vec2(off, off));

    vec3 baseColor = vec3(0.9) * middle + sum * 0.1 * vec3(1.0) * bar;
    return baseColor;
}

vec2 barrel(vec2 uv){
  vec2 c = uv * 2.0 - 1.0;
  float r2 = dot(c, c);
  c *= 1.0 + uCurvature * r2;
  return c * 0.5 + 0.5;
}

void main() {
    time = iTime * 0.333333;
    vec2 uv = vUv;

    if(uCurvature != 0.0){
      uv = barrel(uv);
    }

    vec2 p = uv * uScale;
    vec3 signal = getColor(p);

    if(uChromaticAberration != 0.0){
      vec2 ca = vec2(uChromaticAberration) / iResolution.xy;
      signal.r = getColor(p + ca).r;
      signal.b = getColor(p - ca).b;
    }

    vec3 intensity = clamp(signal * uBrightness, 0.0, 1.0);
    vec3 col = mix(uBackground, uTint, intensity);

    if(uDither > 0.0){
      float rnd = hash21(gl_FragCoord.xy);
      col += (rnd - 0.5) * (uDither * 0.003922);
    }

    gl_FragColor = vec4(col, 1.0);
}
`

function resolveColorValue(color, element) {
  const source = element ?? (typeof document !== 'undefined' ? document.documentElement : null)
  if (typeof color === 'string' && color.trim().startsWith('var(') && source) {
    const start = color.indexOf('--')
    const end = color.lastIndexOf(')')
    if (start !== -1 && end !== -1 && end > start) {
      const variableName = color.slice(start, end).trim()
      const resolved = getComputedStyle(source).getPropertyValue(variableName).trim()
      if (resolved) {
        return resolved
      }
    }
  }

  return color
}

function colorToRgb(color, element) {
  const resolved = resolveColorValue(color, element)

  if (resolved.startsWith('rgb')) {
    const matches = resolved.match(RGB_MATCHER)
    if (matches?.length >= 3) {
      return matches.slice(0, 3).map(value => Number(value) / 255)
    }
  }

  let h = resolved.replace('#', '').trim()
  if (h.length === 3) {
    h = h
      .split('')
      .map(c => c + c)
      .join('')
  }
  const num = Number.parseInt(h, 16)
  return [((num >> 16) & 255) / 255, ((num >> 8) & 255) / 255, (num & 255) / 255]
}

export default function FaultyTerminal({
  scale = 1,
  gridMul = [2, 1],
  digitSize = 1.5,
  timeScale = 0.3,
  pause = false,
  scanlineIntensity = 0.3,
  glitchAmount = 1,
  flickerAmount = 1,
  noiseAmp = 0,
  chromaticAberration = 0,
  dither = 0,
  curvature = 0.2,
  tint = '#ffffff',
  backgroundTint = '#000000',
  mouseReact = true,
  mouseStrength = 0.2,
  dpr = Math.min(window.devicePixelRatio || 1, 2),
  timeOffset,
  pageLoadAnimation = true,
  brightness = 1,
  className,
  style,
  ...rest
}) {
  const containerRef = useRef(null)
  const programRef = useRef(null)
  const rendererRef = useRef(null)
  const meshRef = useRef(null)
  const mouseRef = useRef({ x: 0.5, y: 0.5 })
  const smoothMouseRef = useRef({ x: 0.5, y: 0.5 })
  const frozenTimeRef = useRef(0)
  const rafRef = useRef(0)
  const resizeObserverRef = useRef(null)
  const targetSizeRef = useRef({ width: 0, height: 0 })
  const appliedSizeRef = useRef({ width: 0, height: 0 })
  const loadAnimationStartRef = useRef(0)
  const timeOffsetRef = useRef(timeOffset ?? Math.random() * 100)
  const pauseRef = useRef(pause)
  const timeScaleRef = useRef(timeScale)
  const mouseReactRef = useRef(mouseReact)
  const pageLoadAnimationRef = useRef(pageLoadAnimation)
  const previousPageLoadAnimationRef = useRef(pageLoadAnimation)
  const currentTintRef = useRef([1, 1, 1])
  const targetTintRef = useRef([1, 1, 1])
  const currentBackgroundRef = useRef([0, 0, 0])
  const targetBackgroundRef = useRef([0, 0, 0])
  const currentCurvatureRef = useRef(curvature)
  const targetCurvatureRef = useRef(curvature)
  const currentScanlineRef = useRef(scanlineIntensity)
  const targetScanlineRef = useRef(scanlineIntensity)
  const [resolvedThemeVersion, setResolvedThemeVersion] = useState(0)

  const tintVec = useMemo(() => colorToRgb(tint, containerRef.current), [resolvedThemeVersion, tint])
  const backgroundVec = useMemo(() => colorToRgb(backgroundTint, containerRef.current), [backgroundTint, resolvedThemeVersion])
  const mergedStyle = useMemo(
    () => ({ ...style, backgroundColor: backgroundTint }),
    [style, backgroundTint],
  )

  const ditherValue = useMemo(() => (typeof dither === 'boolean' ? (dither ? 1 : 0) : dither), [dither])

  const handlePointerMove = useCallback((e) => {
    const ctn = containerRef.current
    if (!ctn)
      return
    const rect = ctn.getBoundingClientRect()
    if (!rect.width || !rect.height)
      return
    const x = (e.clientX - rect.left) / rect.width
    const y = 1 - (e.clientY - rect.top) / rect.height
    mouseRef.current = {
      x: Math.min(Math.max(x, 0), 1),
      y: Math.min(Math.max(y, 0), 1),
    }
  }, [])

  useEffect(() => {
    const container = containerRef.current
    if (!container)
      return

    const themedRoot = container.closest('[data-theme]') ?? document.documentElement
    const refreshResolvedColors = () => setResolvedThemeVersion(version => version + 1)

    refreshResolvedColors()

    const observer = new MutationObserver((mutations) => {
      if (mutations.some(mutation =>
        mutation.type === 'attributes'
        && (mutation.attributeName === 'data-theme' || mutation.attributeName === 'data-webview-material'))) {
        refreshResolvedColors()
      }
    })

    observer.observe(themedRoot, {
      attributes: true,
      attributeFilter: ['data-theme', 'data-webview-material'],
    })

    return () => {
      observer.disconnect()
    }
  }, [])

  useEffect(() => {
    pauseRef.current = pause
    timeScaleRef.current = timeScale
    mouseReactRef.current = mouseReact
    pageLoadAnimationRef.current = pageLoadAnimation
  }, [mouseReact, pageLoadAnimation, pause, timeScale])

  useEffect(() => {
    const program = programRef.current
    const wasEnabled = previousPageLoadAnimationRef.current
    previousPageLoadAnimationRef.current = pageLoadAnimation

    if (!program) {
      if (pageLoadAnimation) {
        loadAnimationStartRef.current = 0
      }
      return
    }

    if (!wasEnabled && pageLoadAnimation) {
      loadAnimationStartRef.current = 0
      program.uniforms.uPageLoadProgress.value = 0
      program.uniforms.uUsePageLoadAnimation.value = 1
      return
    }

    if (!pageLoadAnimation) {
      loadAnimationStartRef.current = 0
      program.uniforms.uPageLoadProgress.value = 1
      program.uniforms.uUsePageLoadAnimation.value = 0
    }
  }, [pageLoadAnimation])

  useEffect(() => {
    targetTintRef.current = [...tintVec]
    if (!programRef.current) {
      currentTintRef.current = [...tintVec]
    }
  }, [tintVec])

  useEffect(() => {
    targetBackgroundRef.current = [...backgroundVec]
    if (!programRef.current) {
      currentBackgroundRef.current = [...backgroundVec]
    }
  }, [backgroundVec])

  useEffect(() => {
    targetCurvatureRef.current = curvature
    if (!programRef.current) {
      currentCurvatureRef.current = curvature
    }
  }, [curvature])

  useEffect(() => {
    targetScanlineRef.current = scanlineIntensity
    if (!programRef.current) {
      currentScanlineRef.current = scanlineIntensity
    }
  }, [scanlineIntensity])

  useEffect(() => {
    const ctn = containerRef.current
    if (!ctn)
      return

    let renderer = null
    let program = null
    let mesh = null
    let gl = null
    const cleanupWebgl = () => {
      cancelAnimationFrame(rafRef.current)
      resizeObserverRef.current?.disconnect()
      resizeObserverRef.current = null
      window.removeEventListener('pointermove', handlePointerMove)
      if (gl?.canvas?.parentElement === ctn)
        ctn.removeChild(gl.canvas)
      meshRef.current = null
      programRef.current = null
      rendererRef.current = null
      targetSizeRef.current = { width: 0, height: 0 }
      appliedSizeRef.current = { width: 0, height: 0 }
      gl?.getExtension('WEBGL_lose_context')?.loseContext()
      loadAnimationStartRef.current = 0
    }

    try {
      renderer = new Renderer({ dpr })
      rendererRef.current = renderer
      gl = renderer.gl
      gl.clearColor(
        currentBackgroundRef.current[0],
        currentBackgroundRef.current[1],
        currentBackgroundRef.current[2],
        1,
      )

      const geometry = new Triangle(gl)

      program = new Program(gl, {
        vertex: vertexShader,
        fragment: fragmentShader,
        uniforms: {
          iTime: { value: 0 },
          iResolution: {
            value: new Color(gl.canvas.width, gl.canvas.height, gl.canvas.width / gl.canvas.height),
          },
          uScale: { value: scale },

          uGridMul: { value: new Float32Array(gridMul) },
          uDigitSize: { value: digitSize },
          uScanlineIntensity: { value: currentScanlineRef.current },
          uGlitchAmount: { value: glitchAmount },
          uFlickerAmount: { value: flickerAmount },
          uNoiseAmp: { value: noiseAmp },
          uChromaticAberration: { value: chromaticAberration },
          uDither: { value: ditherValue },
          uCurvature: { value: currentCurvatureRef.current },
          uTint: { value: new Color(currentTintRef.current[0], currentTintRef.current[1], currentTintRef.current[2]) },
          uBackground: { value: new Color(currentBackgroundRef.current[0], currentBackgroundRef.current[1], currentBackgroundRef.current[2]) },
          uMouse: {
            value: new Float32Array([smoothMouseRef.current.x, smoothMouseRef.current.y]),
          },
          uMouseStrength: { value: mouseStrength },
          uUseMouse: { value: mouseReact ? 1 : 0 },
          uPageLoadProgress: { value: pageLoadAnimation ? 0 : 1 },
          uUsePageLoadAnimation: { value: pageLoadAnimation ? 1 : 0 },
          uBrightness: { value: brightness },
        },
      })
      programRef.current = program

      mesh = new Mesh(gl, { geometry, program })
      meshRef.current = mesh
    }
    catch (error) {
      console.error('Failed to initialize FaultyTerminal WebGL:', error)
      cleanupWebgl()
      return
    }

    function applySize(width, height) {
      if (!renderer || !width || !height)
        return
      renderer.setSize(width, height)
      appliedSizeRef.current = { width, height }
      program.uniforms.iResolution.value = new Color(
        gl.canvas.width,
        gl.canvas.height,
        gl.canvas.width / gl.canvas.height,
      )
    }

    function renderCurrentFrame() {
      if (!renderer || !mesh)
        return
      renderer.render({ scene: mesh })
    }

    const updateTargetSize = () => {
      if (!ctn)
        return

      const { width, height } = ctn.getBoundingClientRect()
      targetSizeRef.current = {
        width: Math.max(0, Math.round(width)),
        height: Math.max(0, Math.round(height)),
      }
    }

    updateTargetSize()
    applySize(targetSizeRef.current.width, targetSizeRef.current.height)

    resizeObserverRef.current = new ResizeObserver(() => {
      updateTargetSize()
      const targetSize = targetSizeRef.current
      const appliedSize = appliedSizeRef.current

      if (
        targetSize.width > 0
        && targetSize.height > 0
        && (targetSize.width !== appliedSize.width || targetSize.height !== appliedSize.height)
      ) {
        applySize(targetSize.width, targetSize.height)
        renderCurrentFrame()
      }
    })
    resizeObserverRef.current.observe(ctn)

    const update = (t) => {
      rafRef.current = requestAnimationFrame(update)

      const currentProgram = programRef.current
      const currentRenderer = rendererRef.current
      const currentMesh = meshRef.current
      if (!currentProgram || !currentRenderer || !currentMesh) {
        return
      }

      if (pageLoadAnimationRef.current && loadAnimationStartRef.current === 0) {
        loadAnimationStartRef.current = t
      }

      if (!pauseRef.current) {
        const elapsed = (t * 0.001 + timeOffsetRef.current) * timeScaleRef.current
        currentProgram.uniforms.iTime.value = elapsed
        frozenTimeRef.current = elapsed
      }
      else {
        currentProgram.uniforms.iTime.value = frozenTimeRef.current
      }

      if (pageLoadAnimationRef.current && loadAnimationStartRef.current > 0) {
        const animationDuration = 2000
        const animationElapsed = t - loadAnimationStartRef.current
        const progress = Math.min(animationElapsed / animationDuration, 1)
        currentProgram.uniforms.uPageLoadProgress.value = progress
      }

      if (mouseReactRef.current) {
        const dampingFactor = 0.08
        const smoothMouse = smoothMouseRef.current
        const mouse = mouseRef.current
        smoothMouse.x += (mouse.x - smoothMouse.x) * dampingFactor
        smoothMouse.y += (mouse.y - smoothMouse.y) * dampingFactor

        const mouseUniform = currentProgram.uniforms.uMouse.value
        mouseUniform[0] = smoothMouse.x
        mouseUniform[1] = smoothMouse.y
      }

      const colorDamping = 0.1
      for (let i = 0; i < 3; i++) {
        currentTintRef.current[i] += (targetTintRef.current[i] - currentTintRef.current[i]) * colorDamping
        currentBackgroundRef.current[i] += (targetBackgroundRef.current[i] - currentBackgroundRef.current[i]) * colorDamping
      }
      currentCurvatureRef.current += (targetCurvatureRef.current - currentCurvatureRef.current) * colorDamping
      currentScanlineRef.current += (targetScanlineRef.current - currentScanlineRef.current) * colorDamping
      currentProgram.uniforms.uTint.value = new Color(
        currentTintRef.current[0],
        currentTintRef.current[1],
        currentTintRef.current[2],
      )
      currentProgram.uniforms.uBackground.value = new Color(
        currentBackgroundRef.current[0],
        currentBackgroundRef.current[1],
        currentBackgroundRef.current[2],
      )
      currentProgram.uniforms.uCurvature.value = currentCurvatureRef.current
      currentProgram.uniforms.uScanlineIntensity.value = currentScanlineRef.current

      currentRenderer.render({ scene: currentMesh })
    }
    rafRef.current = requestAnimationFrame(update)
    ctn.appendChild(gl.canvas)

    window.addEventListener('pointermove', handlePointerMove, { passive: true })

    return cleanupWebgl
  }, [dpr, handlePointerMove])

  useEffect(() => {
    const program = programRef.current
    if (!program)
      return

    program.uniforms.uScale.value = scale
    program.uniforms.uGridMul.value = new Float32Array(gridMul)
    program.uniforms.uDigitSize.value = digitSize
    program.uniforms.uGlitchAmount.value = glitchAmount
    program.uniforms.uFlickerAmount.value = flickerAmount
    program.uniforms.uNoiseAmp.value = noiseAmp
    program.uniforms.uChromaticAberration.value = chromaticAberration
    program.uniforms.uDither.value = ditherValue
    program.uniforms.uMouseStrength.value = mouseStrength
    program.uniforms.uUseMouse.value = mouseReact ? 1 : 0
    program.uniforms.uUsePageLoadAnimation.value = pageLoadAnimation ? 1 : 0
    program.uniforms.uBrightness.value = brightness

    if (!pageLoadAnimation) {
      program.uniforms.uPageLoadProgress.value = 1
    }
  }, [
    brightness,
    chromaticAberration,
    curvature,
    digitSize,
    ditherValue,
    flickerAmount,
    glitchAmount,
    gridMul,
    backgroundVec,
    mouseReact,
    mouseStrength,
    noiseAmp,
    pageLoadAnimation,
    scale,
    scanlineIntensity,
    tintVec,
  ])

  return <div ref={containerRef} className={`faulty-terminal-container ${className}`} style={mergedStyle} {...rest} />
}
