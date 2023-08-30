import { Result } from "./types";

type ErrorRecord = { error: unknown }

export interface ErrorRegistry {
    consumeError(id: number): ErrorRecord | undefined
}

export class ErrorRegistryImplementation implements ErrorRegistry {
    private registeredErrors: ErrorRecord[] = []

    consumeError(id: number): { error: unknown; } | undefined {
        return this.registeredErrors[id]
    }

    registerNewError(error: unknown) {
        let i=0;
        while (this.registeredErrors[i] !== undefined) {
            i++
        }
        this.registeredErrors[i] = { error }
        return i
    }

}

export function wrapAsync<A extends unknown[], R>(registry: ErrorRegistryImplementation, fn: (...args: A) => Promise<Result<R>>): (...args: A) => Promise<Result<R>> {
    return async (...args) => {
        try {
            return await fn(...args)
        } catch (error) {
            const id = registry.registerNewError(error)
            return { ok: false, error: { kind: 'JsError', id } }
        }
    }
}
