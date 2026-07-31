export interface PublicKeyBase {
    uid: string;
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
    publicKeyBase: string | null;
    provName: string;
    provNodeId: string;
    provRewardAddr: string;
}

export interface FancySearchResponse {
    entries: Fancy[];
    total: number;
    totalCapped: boolean;
    limit: number;
    offset: number;
}

export interface FancyCategoryStat {
    category: string;
    total: number;
    free: number;
    newest: string | null;
}

export interface FancyStats {
    total: number;
    free: number;
    newest: string | null;
    estimatedWorkTh: number;
    categories: FancyCategoryStat[];
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
