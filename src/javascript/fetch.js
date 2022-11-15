((window) => {
    const core = window.Deno.core;
    const {
        TypedArrayPrototypeSubarray,
        TypeError,
        Uint8Array,
    } = window.__bootstrap.primordials;
    const { ReadableStream } =
        window.__bootstrap.streams;
    const RESOURCE_REGISTRY = new FinalizationRegistry((rid) => {
        core.tryClose(rid);
    });
    function intoResponseHeaders(headers) {
        if (Array.isArray(headers)) {
            let index = 0, len = headers.length;
            const HashMap = {};
            for (; index < len; index++) {
                const [key, value] = headers[index];
                HashMap[key] = value;
            }
            return HashMap;
        } else {
            throw new TypeError("the headers is not array");
        }
    }
    async function fetch(url, options) {
        const method = options.method || "GET";
        const headers = options.headers || {};
        const opHeaders = [];
        for (const key in headers) {
            opHeaders.push([key, headers[key]]);
        }
        let { requestRid, cancelHandleRid } = core.ops.op_fetch(method, url, opHeaders);
        const response = await core.opAsync("op_fetch_send", requestRid);
        const responseBodyRid = response.requestRid

        // 将data拿出来
        const readable = new ReadableStream({
            type: "bytes",
            async pull(controller) {
                try {
                    // This is the largest possible size for a single packet on a TLS
                    // stream.

                    const chunk = new Uint8Array(16 * 1024 + 256);
                    // TODO(@AaronO): switch to handle nulls if that's moved to core
                    // 使用core.read 将数据读取出来 chunk 是每次读多少的意思
                    // 这样做的好处是：当data太大时，不会导致阻塞v8进程
                    // todo: Error: The operation is not supported
                    const read = await core.read(
                        responseBodyRid,
                        chunk,
                    );
                    if (read > 0) {
                        // 如果能读取到数据、就把它加入到队列中
                        // todo: TypedArrayPrototypeSubarray
                        controller.enqueue(TypedArrayPrototypeSubarray(chunk, 0, read));
                    } else {
                        RESOURCE_REGISTRY.unregister(readable);
                        // 如果没有数据了。那就关闭这个流，并在rust中drop掉这块内存
                        controller.close();
                        core.tryClose(responseBodyRid);
                    }
                } catch (err) {
                    RESOURCE_REGISTRY.unregister(readable);
                    controller.error(err);
                    core.tryClose(responseBodyRid);
                }
            }
        })


        return {
            ok: response.status === 200,
            status: response.status,
            statusText: response.status === 200 ? "OK" : "NO",
            url: response.url,
            headers: intoResponseHeaders(response.headers),
            text: async () => {
                const reader = readable.getReader();
                const chunks = [];
                let totalLength = 0;
                while (true) {
                    const { value: chunk, done } = await reader.read();
                    if (done) break;
                    chunks.push(chunk)
                    totalLength += chunk.byteLength;
                }
                const finalBuffer = new Uint8Array(totalLength);
                let i = 0;
                for (const chunk of chunks) {
                    finalBuffer.set(chunk, i);
                    i += chunk.byteLength;
                }
                return core.decode(finalBuffer)
            },
            json: () => {
                // return JSON.parse(core.decode(finalBuffer))
            },
            responseBodyRid
            // body: response.body
            // prototype
            // bodyUsed
        }
    }
    window.fetch = fetch;
})(this)