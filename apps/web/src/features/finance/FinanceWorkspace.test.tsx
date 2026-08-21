// @vitest-environment jsdom
import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

const testState = vi.hoisted(() => ({
  mountCount: 0,
  session: {
    user: { id: "user-a" },
    session: { id: "session-a" },
  } as { user: { id: string }; session: { id: string } } | null,
}));

vi.mock("../../app/AppContext", () => ({
  useApp: () => ({ session: testState.session }),
}));

vi.mock("./beecount-cloud/BeeCountCloudWorkspace", async () => {
  const React = await import("react");
  return {
    BeeCountCloudWorkspace: () => {
      const [instance] = React.useState(() => ++testState.mountCount);
      return <div data-testid="finance-instance">{instance}</div>;
    },
  };
});

import { FinanceWorkspace } from "./FinanceWorkspace";

describe("FinanceWorkspace session isolation", () => {
  it("destroys the BeeCount workspace when the authenticated account changes", () => {
    testState.mountCount = 0;
    testState.session = {
      user: { id: "user-a" },
      session: { id: "session-a" },
    };
    const view = render(<FinanceWorkspace />);
    expect(screen.getByTestId("finance-instance")).toHaveTextContent("1");

    testState.session = {
      user: { id: "user-b" },
      session: { id: "session-b" },
    };
    view.rerender(<FinanceWorkspace />);

    expect(screen.getByTestId("finance-instance")).toHaveTextContent("2");
  });

  it("also destroys cached finance state when the session rotates for the same user", () => {
    testState.mountCount = 0;
    testState.session = {
      user: { id: "user-a" },
      session: { id: "session-a" },
    };
    const view = render(<FinanceWorkspace />);
    expect(screen.getByTestId("finance-instance")).toHaveTextContent("1");

    testState.session = {
      user: { id: "user-a" },
      session: { id: "session-a-rotated" },
    };
    view.rerender(<FinanceWorkspace />);

    expect(screen.getByTestId("finance-instance")).toHaveTextContent("2");
  });
});
