
import assert from "node:assert/strict";
import test from "node:test";
import { isActionDisabled, resolveActions } from "../src/ui/actions/resolveActions";
import type { AppAction } from "../src/ui/actions/types";

interface Context { archived: boolean; editable: boolean }

const actions: AppAction<Context>[] = [
  { id: "open", label: "Open", execute: () => undefined },
  { id: "edit", label: "Edit", hidden: (context) => !context.editable, execute: () => undefined },
  { id: "archive", label: "Archive", disabled: (context) => context.archived, execute: () => undefined },
];

test("filters hidden actions from a context", () => {
  assert.deepEqual(resolveActions(actions, { archived: false, editable: false }).map((action) => action.id), ["open", "archive"]);
});

test("resolves disabled state from a context", () => {
  const archive = actions[2];
  assert.equal(isActionDisabled(archive, { archived: true, editable: true }), true);
  assert.equal(isActionDisabled(archive, { archived: false, editable: true }), false);
});
