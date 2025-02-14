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
    miner: string | null;
    mined: string | null;
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
