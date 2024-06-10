import { TransactionTypes, beforeEach, describeSuite, expect } from "@moonwall/cli";
import { ALITH_ADDRESS, BALTATHAR_ADDRESS, MIN_GAS_PRICE, createRawTransfer } from "@moonwall/util";
import { PrivateKeyAccount } from "viem";
import { generatePrivateKey, privateKeyToAccount } from "viem/accounts";

describeSuite({
    id: "DF0101",
    title: "Existential Deposit disabled",
    foundationMethods: "dev",
    testCases: ({ context, it }) => {
        console.log("hello")
    }
});
