/**
 * fetch-based SSE client.
 *
 * Encapsulates the transport concerns of consuming a Server-Sent Events
 * stream: issuing the request, reading the chunked body, parsing SSE
 * frames, and reconnecting with exponential backoff. The client stays
 * free of any status-specific business logic — it only exposes the HTTP
 * status via `onResponse` so callers can react (e.g. to 401) themselves.
 */

export interface SseClientOptions {
  /** Called for every dispatched SSE event (joined `data:` payload). */
  onMessage: (data: string) => void
  /** Called once the stream is open and ready to deliver events. */
  onOpen?: () => void
  /** Called when the stream ends (server close or transient error). */
  onClose?: () => void
  /**
   * Called with the HTTP status code as soon as the response arrives,
   * before the body is consumed. If the caller disconnects the client
   * from within this callback, the client stops (no reconnect).
   */
  onResponse?: (status: number) => void
  /** Initial reconnect delay in ms. Defaults to 2000. */
  minDelay?: number
  /** Max reconnect delay in ms. Defaults to 30000. */
  maxDelay?: number
}

export class SseClient {
  private readonly opts: SseClientOptions
  private readonly minDelay: number
  private readonly maxDelay: number

  private controller: AbortController | null = null
  private timer: ReturnType<typeof setTimeout> | null = null
  private delay: number
  private url = ""

  constructor(opts: SseClientOptions) {
    this.opts = opts
    this.minDelay = opts.minDelay ?? 2000
    this.maxDelay = opts.maxDelay ?? 30000
    this.delay = this.minDelay
  }

  /** Start (or resume) consuming the stream at the given URL. */
  connect(url: string): void {
    if (this.controller || this.timer) return
    this.url = url
    void this.openStream()
  }

  /** Abort the active stream and cancel any pending reconnect. */
  disconnect(): void {
    this.clearTimer()
    if (this.controller) {
      this.controller.abort()
      this.controller = null
    }
  }

  private async openStream(): Promise<void> {
    const controller = new AbortController()
    this.controller = controller

    try {
      const res = await fetch(this.url, {
        method: "GET",
        headers: { Accept: "text/event-stream" },
        signal: controller.signal,
      })

      // Expose the HTTP status so the caller can react (e.g. to 401).
      // If the caller disconnects from within the callback, bail out.
      this.opts.onResponse?.(res.status)
      if (this.controller !== controller) return

      if (!res.ok || !res.body) {
        this.reset(controller)
        this.opts.onClose?.()
        this.scheduleReconnect()
        return
      }

      this.delay = this.minDelay
      this.opts.onOpen?.()
      await this.readStream(res.body, controller)

      // Stream closed by the server — reconnect with the last URL.
      if (this.controller === controller) {
        this.controller = null
        this.opts.onClose?.()
        this.scheduleReconnect()
      }
    } catch {
      if (controller.signal.aborted) return
      this.reset(controller)
      this.opts.onClose?.()
      this.scheduleReconnect()
    }
  }

  private async readStream(
    body: ReadableStream<Uint8Array>,
    controller: AbortController,
  ): Promise<void> {
    const reader = body.getReader()
    const decoder = new TextDecoder()
    let buffer = ""
    let dataLines: string[] = []

    const flush = () => {
      if (dataLines.length) {
        this.opts.onMessage(dataLines.join("\n"))
        dataLines = []
      }
    }

    const processLine = (line: string) => {
      if (line === "") {
        flush()
        return
      }
      if (line.startsWith(":")) return // comment / keep-alive
      if (line.startsWith("data:")) {
        dataLines.push(line.slice(5).replace(/^ /, ""))
      }
      // event:/id:/retry: are ignored — caller treats all as messages.
    }

    for (;;) {
      const { value, done } = await reader.read()
      if (done) break
      if (this.controller !== controller) {
        reader.cancel().catch(() => {})
        return
      }
      buffer += decoder.decode(value, { stream: true })

      let nl: number
      while ((nl = buffer.indexOf("\n")) >= 0) {
        const line = buffer.slice(0, nl).replace(/\r$/, "")
        buffer = buffer.slice(nl + 1)
        processLine(line)
      }
    }

    if (buffer.length) processLine(buffer)
    flush()
  }

  private reset(controller: AbortController): void {
    if (this.controller === controller) {
      this.controller = null
    }
  }

  private scheduleReconnect(): void {
    if (this.controller || this.timer) return

    this.timer = setTimeout(() => {
      this.timer = null
      void this.openStream()
    }, this.delay)
    this.delay = Math.min(this.delay * 2, this.maxDelay)
  }

  private clearTimer(): void {
    if (this.timer) {
      clearTimeout(this.timer)
      this.timer = null
    }
  }
}
