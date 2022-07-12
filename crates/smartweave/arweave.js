(function(window) {

    class BaseObject {
        get(field, options) {
            if (!Object.getOwnPropertyNames(this).includes(field)) {
                throw new Error(`Field "${field}" is not a property of the Arweave Transaction class.`);
            }
            // Handle fields that are Uint8Arrays.
            // To maintain compat we encode them to b64url
            // if decode option is not specificed.
            if (this[field] instanceof Uint8Array) {
                if (options && options.decode && options.string) {
                    return ArweaveUtils.bufferToString(this[field]);
                }
                if (options && options.decode && !options.string) {
                    return this[field];
                }
                return ArweaveUtils.bufferTob64Url(this[field]);
            }
            if (options && options.decode == true) {
                if (options && options.string) {
                    return ArweaveUtils.b64UrlToString(this[field]);
                }
                return ArweaveUtils.b64UrlToBuffer(this[field]);
            }
            return this[field];
        }
    }

    window.BaseObject = BaseObject;

    class Tag extends BaseObject {
        constructor(name, value, decode = false) {
            super();
            this.name = name;
            this.value = value;
        }
    }

    window.Tag = Tag;

    class Transaction extends BaseObject {
        constructor(attributes = {}) {
            super();
            Object.assign(this, attributes);

            if (typeof this.data === "string") {
                this.data = ArweaveUtils.b64UrlToBuffer(this.data);
            }

            if (attributes.tags) {
                this.tags = attributes.tags.map((tag) => new Tag(tag.name, tag.value));
            }
        }

        addTag(name, value) {
            this.tags.push(
                new Tag(
                    ArweaveUtils.stringToB64Url(name),
                    ArweaveUtils.stringToB64Url(value)
                )
            );
        }

        toJSON() {
            return {
                format: this.format,
                id: this.id,
                last_tx: this.last_tx,
                owner: this.owner,
                tags: this.tags,
                target: this.target,
                quantity: this.quantity,
                data: ArweaveUtils.bufferTob64Url(this.data),
                data_size: this.data_size,
                data_root: this.data_root,
                data_tree: this.data_tree,
                reward: this.reward,
                signature: this.signature,
            };
        }
    }

    window.Transaction = Transaction;
})(this);