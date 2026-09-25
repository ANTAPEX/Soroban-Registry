import { act } from "react";
import { createRoot } from "react-dom/client";
import { useFormValidation } from "@/lib/formValidation";

type Values = { name: string };
type Hook = ReturnType<typeof useFormValidation<Values>>;

// Minimal harness: renders the hook and exposes its latest return value.
function mountHook() {
  const ref: { current: Hook | null } = { current: null };
  function Probe() {
    ref.current = useFormValidation<Values>({
      initialValues: { name: "" },
      validate: (v) => ({ name: v.name ? undefined : "Name is required" }),
      onSubmit: () => {},
      debounceMs: 0,
    });
    return null;
  }
  const container = document.createElement("div");
  const root = createRoot(container);
  act(() => root.render(<Probe />));
  return { ref, unmount: () => act(() => root.unmount()) };
}

const flush = () => act(() => new Promise((r) => setTimeout(r, 5)));

beforeAll(() => {
  (globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;
});

test("hides errors for untouched fields on a fresh form", async () => {
  const { ref, unmount } = mountHook();
  await flush();
  expect(ref.current!.errors.name).toBeUndefined();
  unmount();
});

test("shows a field's error after it is blurred", async () => {
  const { ref, unmount } = mountHook();
  await flush();
  act(() => ref.current!.handleBlur({ target: { name: "name" } } as React.FocusEvent<HTMLInputElement>));
  await flush();
  expect(ref.current!.errors.name).toBe("Name is required");
  unmount();
});

test("shows every error after a submit attempt", async () => {
  const { ref, unmount } = mountHook();
  await flush();
  await act(async () => {
    await ref.current!.handleSubmit();
  });
  expect(ref.current!.errors.name).toBe("Name is required");
  unmount();
});
