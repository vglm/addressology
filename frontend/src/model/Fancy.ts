export interface PublicKeyBase {
    id: string;
    hex: string;
    added: string;
    user_id: string | null;
}

export interface FancyScore {
    score: {
        addressLowerCase: string;
        addressMixedCase: string;
        addressShortEtherscan: string;
        scores: {
            [key: string]: {
                score: number;
                difficulty: number;
                category: string;
            };
        };
        totalScore: number;
        price: number;
        category: string;
    };
    price: number;
    minerInfo: {
        provNodeId: string;
        provRewardAddr: string;
        provName: string;
        provExtraInfo: string;
    } | null;
    mined: string | null;
    publicKeyBase: string | null;
    factory: string | null;
    salt: string | null;
}

export interface Fancy {
    address: string;
    score: number;
    price: number;
    category: string;
    created: string;
    job: string | null;
    provName: string;
    provNodeId: string;
    provRewardAddr: string;
}

export interface UserTokenResponse {
    uid: string;
    email: string;
    tokens: number;
}

export interface FancyCategoryInfo {
    key: string;
    name: string;
    description: string;
}
