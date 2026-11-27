import { describe, expect, it } from "vite-plus/test";
import { getPreviewMessages, previewText } from "./preview.ts";

describe("getPreviewMessages", () => {
  it("includes rich messages and both conditional branches in workspace order", () => {
    expect(
      getPreviewMessages({
        handlers: [
          {
            trigger: {
              type: "slash_command",
              name: "welcome",
              description: "Welcome",
              options: [],
            },
            actions: [
              {
                type: "if",
                condition: { type: "boolean", value: true },
                then: [
                  {
                    type: "reply",
                    content: "Hi {user}",
                    ephemeral: true,
                    embeds: [
                      {
                        title: "Welcome",
                        fields: [],
                        timestamp: false,
                      },
                    ],
                  },
                ],
                else: [{ type: "direct_message", member_id: "1", content: "Please try again" }],
              },
            ],
          },
        ],
      }),
    ).toEqual([
      {
        handlerIndex: 0,
        trigger: "/welcome",
        kind: "reply",
        content: "Hi {user}",
        embeds: [{ title: "Welcome", fields: [], timestamp: false }],
        components: undefined,
        ephemeral: true,
        condition: "when the condition is met",
      },
      {
        handlerIndex: 0,
        trigger: "/welcome",
        kind: "direct message",
        content: "Please try again",
        ephemeral: false,
        condition: "otherwise",
      },
    ]);
  });
});

describe("previewText", () => {
  it("renders placeholders with sample Discord data", () => {
    expect(
      previewText(
        "Hi {user.name}: {option.reason} #{member:points} {selected} [{modal.topic}] -> {target}",
      ),
    ).toBe("Hi SampleUser: example #42 First option [Submitted text] -> @TargetUser");
  });
});
