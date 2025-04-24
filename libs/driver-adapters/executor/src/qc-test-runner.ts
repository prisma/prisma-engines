import * as events from 'node:events'
import * as readline from 'node:readline'
import * as util from 'node:util'
import { Worker, MessageChannel } from 'node:worker_threads'
import * as S from '@effect/schema/Schema'

import { Env, jsonRpc } from './types'
import { assertNever, debug, err } from './utils'
import { SchemaId } from './types/jsonRpc'
import type { Message } from './qc-test-worker/worker'

async function main(): Promise<void> {
  const env = S.decodeUnknownSync(Env)(process.env)
  // console.log('[env]', env)

  const iface = readline.createInterface({
    input: process.stdin,
    output: process.stdout,
    terminal: false,
  })

  iface.on('line', async (line) => {
    try {
      const request = S.decodeSync(jsonRpc.RequestFromString)(line)
      debug(`Got a request: ${line}`)

      try {
        const response = await handleRequest(request, env)
        respondOk(request.id, response)
      } catch (err) {
        debug('[nodejs] Error from request handler: ', err)
        respondErr(request.id, {
          code: 1,
          message: err.stack ?? err.toString(),
        })
      }
    } catch (err) {
      debug('Received non-json line: ', line)
      console.error(err)
    }
  })
}

const state: Record<
  SchemaId,
  {
    worker: Worker
    health: { status: 'running' } | { status: 'terminated'; error: Error }
  }
> = {}

async function handleRequest(
  { method, params }: jsonRpc.Request,
  env: Env,
): Promise<unknown> {
  const schemaState = state[params.schemaId]

  if (schemaState?.health.status === 'terminated') {
    throw schemaState.health.error
  }

  if (method !== 'initializeSchema') {
    if (schemaState === undefined) {
      throw new Error(
        `Schema with id ${params.schemaId} is not initialized. Please call 'initializeSchema' first.`,
      )
    }
  }

  switch (method) {
    case 'initializeSchema': {
      debug('Got `initializeSchema`', params)

      const worker = new Worker(
        new URL('qc-test-worker/worker.js', import.meta.url),
      )

      worker.unref()

      state[params.schemaId] = {
        worker,
        health: { status: 'running' },
      }

      const schemaState = state[params.schemaId]

      worker.on('error', (error) => {
        console.error('Worker error:', error)
        schemaState.health = { status: 'terminated', error }
      })

      return await messageWorker(schemaState.worker, {
        type: 'initializeSchema',
        params,
        env,
      })
    }

    case 'query': {
      debug('Got `query`', util.inspect(params, false, null, true))

      return await messageWorker(schemaState.worker, {
        type: 'query',
        params,
      })
    }

    case 'startTx': {
      debug('Got `startTx`', params)

      return await messageWorker(schemaState.worker, {
        type: 'startTx',
        params,
      })
    }

    case 'commitTx': {
      debug('Got `commitTx`', params)

      return await messageWorker(schemaState.worker, {
        type: 'commitTx',
        params,
      })
    }

    case 'rollbackTx': {
      debug('Got `rollbackTx`', params)

      return await messageWorker(schemaState.worker, {
        type: 'rollbackTx',
        params,
      })
    }

    case 'teardown': {
      debug('Got `teardown`', params)

      try {
        await messageWorker(schemaState.worker, {
          type: 'teardown',
        })
      } finally {
        await schemaState.worker.terminate()
      }

      return {}
    }

    case 'getLogs': {
      debug('Got `getLogs`', params)

      return await messageWorker(schemaState.worker, {
        type: 'getLogs',
      })
    }

    default: {
      assertNever(method, `Unknown method: \`${method}\``)
    }
  }
}

function respondErr(requestId: number, error: jsonRpc.RpcError) {
  const msg: jsonRpc.ErrResponse = {
    jsonrpc: '2.0',
    id: requestId,
    error,
  }
  console.log(JSON.stringify(msg))
}

function respondOk(requestId: number, payload: unknown) {
  const msg: jsonRpc.OkResponse = {
    jsonrpc: '2.0',
    id: requestId,
    result: payload,
  }
  console.log(JSON.stringify(msg))
}

type MessageWithoutResponsePort = {
  [K in Message['type']]: Omit<Extract<Message, { type: K }>, 'responsePort'>
}[Message['type']]

async function messageWorker(
  worker: Worker,
  message: MessageWithoutResponsePort,
): Promise<unknown> {
  const { port1, port2 } = new MessageChannel()
  const responsePromise = events.once(port1, 'message')

  worker.postMessage(
    {
      ...message,
      responsePort: port2,
    },
    [port2],
  )

  const [response] = await responsePromise

  if (response instanceof Error) {
    throw response
  }

  return response
}

main().catch(err)
