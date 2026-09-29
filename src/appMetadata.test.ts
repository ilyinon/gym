import { describe, expect, it } from "vitest";
import { APP_NAME } from "./appMetadata";

describe("app metadata", () => {
  it("uses the product name from the specification", () => {
    expect(APP_NAME).toBe("Gym");
  });
});
