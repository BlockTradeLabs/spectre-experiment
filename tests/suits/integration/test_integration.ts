import { TransactionTypes, beforeEach, describeSuite, expect } from "@moonwall/cli";
import { ALITH_ADDRESS, BALTATHAR_ADDRESS, MIN_GAS_PRICE, createRawTransfer } from "@moonwall/util";
import { PrivateKeyAccount } from "viem";
import { generatePrivateKey, privateKeyToAccount } from "viem/accounts";
import { describe, beforeAll, it } from "vitest";


describe("basic", () => {
    beforeAll(() => {
      console.log("running this before tests");
    });
  
    it("should run", () => {
      expect(true).toBe(true);
    });
  
    it("should run", () => {
      expect(3 > 2).toBe(true);
    });
    it("should run", () => {
      expect("true".length).toBeGreaterThan(0);
    });
  });