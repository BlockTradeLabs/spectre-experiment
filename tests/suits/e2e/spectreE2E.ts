import { TransactionTypes, beforeEach, describeSuite, expect } from "@moonwall/cli";
import { ALITH_ADDRESS, BALTATHAR_ADDRESS, MIN_GAS_PRICE, createRawTransfer } from "@moonwall/util";
import { PrivateKeyAccount } from "viem";
import { generatePrivateKey, privateKeyToAccount } from "viem/accounts";

describeSuite({
    id: "END TO END MVP",
    title: "TEST INTEGRATION ON SPECTRE NODE, PHALA CONTRACT AND USER SIMULATED INTERACTION",
    foundationMethods: "zombie",
    testCases: ({ context, it }) => {
               

    },
});
