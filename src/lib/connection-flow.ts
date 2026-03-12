import { useConnectionStore } from '@/stores/connection.store'

export async function waitForConnectionStatus(
  expectedStatus: 'connected' | 'disconnected',
  timeoutMs = 15000,
): Promise<void> {
  return new Promise((resolve, reject) => {
    const currentStatus = useConnectionStore.getState().status
    if (currentStatus === expectedStatus) {
      resolve()
      return
    }
    if (currentStatus === 'error') {
      reject(new Error(`Connection entered error state while waiting for ${expectedStatus}`))
      return
    }

    let unsubscribe = () => {}
    const timeoutId = window.setTimeout(() => {
      unsubscribe()
      reject(new Error(`Timeout waiting for connection status: ${expectedStatus}`))
    }, timeoutMs)

    unsubscribe = useConnectionStore.subscribe((state) => {
      if (state.status === expectedStatus) {
        window.clearTimeout(timeoutId)
        unsubscribe()
        resolve()
      }
      else if (state.status === 'error') {
        window.clearTimeout(timeoutId)
        unsubscribe()
        reject(new Error(`Connection entered error state while waiting for ${expectedStatus}`))
      }
    })
  })
}

export async function waitForTerminalConnectionStatus(timeoutMs = 15000): Promise<'connected' | 'disconnected'> {
  return new Promise((resolve, reject) => {
    const currentStatus = useConnectionStore.getState().status
    if (currentStatus === 'connected' || currentStatus === 'disconnected') {
      resolve(currentStatus)
      return
    }
    if (currentStatus === 'error') {
      reject(new Error('Connection entered error state while waiting for terminal status'))
      return
    }

    let unsubscribe = () => {}
    const timeoutId = window.setTimeout(() => {
      unsubscribe()
      reject(new Error('Timeout waiting for terminal connection status'))
    }, timeoutMs)

    unsubscribe = useConnectionStore.subscribe((state) => {
      if (state.status === 'connected' || state.status === 'disconnected') {
        window.clearTimeout(timeoutId)
        unsubscribe()
        resolve(state.status)
      }
      else if (state.status === 'error') {
        window.clearTimeout(timeoutId)
        unsubscribe()
        reject(new Error('Connection entered error state while waiting for terminal status'))
      }
    })
  })
}

export async function runWithPausedConnection(task: () => Promise<void>): Promise<void> {
  const { connect, disconnect } = useConnectionStore.getState()
  let shouldReconnect = false

  let stableStatus = useConnectionStore.getState().status
  if (stableStatus === 'connecting' || stableStatus === 'disconnecting') {
    stableStatus = await waitForTerminalConnectionStatus()
  }

  if (stableStatus === 'connected') {
    shouldReconnect = true
    await disconnect()
    await waitForConnectionStatus('disconnected')
  }

  try {
    await task()
  }
  finally {
    if (shouldReconnect) {
      await connect()
      await waitForConnectionStatus('connected')
    }
  }
}
