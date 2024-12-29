export function createRNEngineConnector(
  url: string,
  schema: string,
  logCallback: (msg: string) => void,
) {
  const headers = {
    'Content-Type': 'application/json',
    Accept: 'application/json',
  }

  return {
    connect: async () => {
      const res = await fetch(`${url}/connect`, {
        method: 'POST',
        mode: 'no-cors',
        headers,
        body: JSON.stringify({ schema }),
      })

      return await res.json()
    },
    query: async (
      body: string,
      trace: string,
      txId: string,
    ): Promise<string> => {
      const res = await fetch(`${url}/query`, {
        method: 'POST',
        mode: 'no-cors',
        headers,
        body: JSON.stringify({
          body,
          trace,
          txId,
        }),
      })

      const response = await res.json()

      if (response.logs.length) {
        response.logs.forEach(logCallback)
      }

      return response.engineResponse
    },
    startTransaction: async (body: string, trace: string): Promise<string> => {
      const res = await fetch(`${url}/start_transaction`, {
        method: 'POST',
        mode: 'no-cors',
        headers,
        body: JSON.stringify({
          body,
          trace,
        }),
      })
      return await res.json()
    },
    commitTransaction: async (txId: string, trace: string): Promise<string> => {
      const res = await fetch(`${url}/commit_transaction`, {
        method: 'POST',
        mode: 'no-cors',
        headers,
        body: JSON.stringify({
          txId,
          trace,
        }),
      })
      return res.json()
    },
    rollbackTransaction: async (
      txId: string,
      trace: string,
    ): Promise<string> => {
      const res = await fetch(`${url}/rollback_transaction`, {
        method: 'POST',
        mode: 'no-cors',
        headers,
        body: JSON.stringify({
          txId,
          trace,
        }),
      })
      return res.json()
    },
    disconnect: async (trace: string) => {
      await fetch(`${url}/disconnect`, {
        method: 'POST',
        mode: 'no-cors',
        headers,
        body: JSON.stringify({
          trace,
        }),
      })
    },
  }
}
