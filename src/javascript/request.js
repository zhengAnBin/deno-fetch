
((window) => {
    function intoHeadersList(headers) {
        const headersList = []
        for (const key in headers) {
            headersList.push([key, headers[key]])
        }
        return headersList;
    }

    function intoHeadersMap(headers) {
        let i = 0
        const len = headers.length;
        const map = {}
        for (; i < len; i++) {
            const [key, value] = headers[i]
            map[key] = value
        }
        return map
    }

    class Request {
        headers
        method
        url
        body
        constructor(input, init = {}) {
            const method = init.method || "GET";
            if (
                (method === "GET" || method === "HEAD") &&
                (init.body !== undefined && init.body !== null)
            ) {
                throw new TypeError("Request with GET/HEAD method cannot have body.");
            }

            this.url = input;
            this.method = method;
            this.headers = init.headers || {}
            this.body = JSON.parse(init.body)
        }
    }

    window.Request = Request;
})(this)