export type FeeType = {
    winston: string,
    ar: string
}

export type Tag = { name: string, value: string};

export const getDefaultFee = (): FeeType => {
    return {
        winston: "",
        ar: ""
    }
}

export const generateFakeInteraction = (input: { [key: string]: any },
                                        id: string,
                                        blockId: string | undefined,
                                        blockHeight: number | undefined,
                                        ownerAddress: string,
                                        recipient: string,
                                        extraTag: Tag | undefined,
                                        quantity: FeeType,
                                        fee: FeeType,
                                        blockTimestamp: number | undefined) => {
    const tags: Array<Tag> = [{
        name: "Input",
        value: JSON.stringify(input)
    }];

    if(extraTag) {
        tags.push(extraTag);
    }

    return {
        cursor: "",
        node: {
            id,
            anchor: undefined,
            signature: undefined,
            recipient,
            owner: {
                address: ownerAddress
            },
            fee,
            quantity,
            data: undefined,
            tags,
            block: {
                id: blockId || "",
                timestamp: blockTimestamp || 0,
                height: blockHeight || 0,
                previous: undefined
            },
            parent: undefined,
            bundledIn: undefined
        }
    }
}
