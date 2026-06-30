import { describe, it, expect } from "vitest";
import { moveItem, restoreOrder } from "./reorder";

describe("reorder helpers", () => {
  it("moveItem moves an element later without mutating the input", () => {
    const arr = ["a", "b", "c", "d"];
    expect(moveItem(arr, 0, 2)).toEqual(["b", "c", "a", "d"]);
    expect(arr).toEqual(["a", "b", "c", "d"]); // input untouched
  });

  it("moveItem moves an element earlier", () => {
    expect(moveItem(["a", "b", "c", "d"], 3, 1)).toEqual(["a", "d", "b", "c"]);
  });

  it("moveItem is a no-op when from === to", () => {
    expect(moveItem(["a", "b", "c"], 1, 1)).toEqual(["a", "b", "c"]);
  });

  it("restoreOrder reorders items to match the id snapshot", () => {
    const items = [{ id: "2" }, { id: "3" }, { id: "1" }];
    expect(restoreOrder(items, ["1", "2", "3"])).toEqual([{ id: "1" }, { id: "2" }, { id: "3" }]);
  });

  it("restoreOrder ignores unknown ids and drops items missing from the snapshot", () => {
    const items = [{ id: "1" }, { id: "2" }];
    expect(restoreOrder(items, ["2", "9", "1"])).toEqual([{ id: "2" }, { id: "1" }]);
  });
});
