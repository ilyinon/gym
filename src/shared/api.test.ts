import { describe, expect, it } from "vitest";
import { errorMessage } from "./api";

describe("backend error presentation", () => {
  it("extracts structured command errors", () => {
    expect(
      errorMessage({ code: "validation_error", message: "Проверьте значение" }),
    ).toBe("Проверьте значение");
    expect(errorMessage("Ошибка соединения")).toBe("Ошибка соединения");
    expect(errorMessage(null)).toBe("Неизвестная ошибка приложения");
  });
});
