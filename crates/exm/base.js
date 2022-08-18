(function(window) {

    const { subtle } = crypto;
    const fetchOp = window.__bootstrap.fetch;

    const props = {
        Request: fetchOp.Request,
        Response: fetchOp.Response,
        fetch: fetchOp.fetch
    }

    window.Request = props.Request;
    window.Response = props.Response;

    class BaseReqResponse {

        #encoder = new TextEncoder();
        #decoder = new TextDecoder();

        buffer = undefined;

        constructor(rep) {
            const { type, url, statusText, status, redirected, ok, headers } = rep;
            this.type = type;
            this.url = url;
            this.statusText = statusText;
            this.status = status;
            this.redirected = redirected;
            this.ok = ok;
            this.headers = headers;
        }

        setBuffer(buff) {
            if(!this.buffer) {
                this.buffer = new Uint8Array(buff);
            } else {
                throw new Error("Buffer already set in Base Request Response");
            }
            return this;
        }

        asText() {
            return this.#decoder.decode(this.buffer);
        }

        asJSON() {
            const text = this.asText();
            return JSON.parse(text);
        }

        asVector() {
            return Object.values(this.buffer);
        }

        toStructuredJson() {
            const { type, url, statusText, status, redirected, ok, headers } = this;
            return {
                type,
                url,
                statusText,
                status,
                redirected,
                ok,
                headers,
                vector: this.asVector()
            }
        }

        get raw() {
            this.arrayBuffer;
        }
    }

    window.BaseReqResponse = BaseReqResponse;

    class Base {

        requests = {};

        async deterministicFetch(...args) {
            try {
                const jsonArgs = JSON.stringify(args);
                const reqHash = await this.sha256(new TextEncoder().encode(jsonArgs));

                const fetchData = await props.fetch(...args);
                // const buff = await fetchData.arrayBuffer();

                // let rep = new BaseReqResponse(fetchData);
                // rep = rep.setBuffer(buff);

                // this.requests[reqHash] = rep.toStructuredJson();

                // return rep;
            } catch (e) {
                return e.toString()
            }
        }

        async sha256(buffer) {
            return subtle.digest('SHA-256', buffer).then((hashBuffer) => {
                const hashArray = Array.from(new Uint8Array(hashBuffer));
                const hashHex = hashArray
                    .map((bytes) => bytes.toString(16).padStart(2, '0'))
                    .join('');
                return hashHex;
            });
        }

    }

    window.Base = new Base();

})(this);