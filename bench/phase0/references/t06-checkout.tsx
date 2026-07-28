import { useState } from "react";

const COUNTRIES = ["GB", "US", "DE"] as const;
type Country = (typeof COUNTRIES)[number];

const LINE_ITEMS = [
  { id: "kbd", name: "Split keyboard", price: 24.0 },
  { id: "cab", name: "Braided cable, 2m", price: 9.5 },
  { id: "cse", name: "Travel case", price: 18.0 },
];

const FREE_SHIPPING_OVER = 50;
const SHIPPING_FLAT = 4.99;

const money = (n: number) => `£${n.toFixed(2)}`;

type Address = {
  fullName: string;
  line1: string;
  line2: string;
  city: string;
  postcode: string;
  country: Country;
};

export default function ShippingStep() {
  const [address, setAddress] = useState<Address>({
    fullName: "",
    line1: "",
    line2: "",
    city: "",
    postcode: "",
    country: "GB",
  });
  const [billingSame, setBillingSame] = useState(true);
  const [pending, setPending] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const subtotal = LINE_ITEMS.reduce((sum, item) => sum + item.price, 0);
  const freeShipping = subtotal > FREE_SHIPPING_OVER;
  const shipping = freeShipping ? 0 : SHIPPING_FLAT;
  const total = subtotal + shipping;

  // Line 2 is the only optional field, so it is excluded rather than special-cased
  // at the point of validation.
  const complete = (["fullName", "line1", "city", "postcode"] as const).every(
    (key) => address[key].trim() !== "",
  );

  function set<K extends keyof Address>(key: K, value: Address[K]) {
    setAddress((prev) => ({ ...prev, [key]: value }));
  }

  async function submit() {
    if (!complete || pending) return;
    setPending(true);
    setError(null);
    try {
      const res = await fetch("/api/checkout/shipping", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ ...address, billingSame }),
      });
      if (!res.ok) throw new Error(`Request failed: ${res.status}`);
    } catch (e) {
      setError(e instanceof Error ? e.message : "Could not save your address");
    } finally {
      setPending(false);
    }
  }

  const fields: Array<{ key: keyof Address; label: string; optional?: boolean; type?: string }> = [
    { key: "fullName", label: "Full name" },
    { key: "line1", label: "Address line 1" },
    { key: "line2", label: "Address line 2", optional: true },
    { key: "city", label: "City" },
    { key: "postcode", label: "Postcode" },
  ];

  return (
    <main className="mx-auto grid max-w-4xl gap-8 px-6 py-10 md:grid-cols-[1.5fr_1fr]">
      <section>
        <h1 className="text-2xl font-semibold text-slate-900">Shipping details</h1>
        <p className="mt-1 text-sm text-slate-500">Step 2 of 3</p>

        {error && (
          <p role="alert" className="mt-4 rounded-lg bg-red-50 px-4 py-3 text-sm text-red-700">
            {error}
          </p>
        )}

        <form
          className="mt-6 space-y-4"
          onSubmit={(e) => {
            e.preventDefault();
            submit();
          }}
        >
          {fields.map(({ key, label, optional }) => (
            <div key={key}>
              <label htmlFor={key} className="block text-sm text-slate-700">
                {label}
                {optional && <span className="text-slate-400"> (optional)</span>}
              </label>
              <input
                id={key}
                required={!optional}
                value={address[key]}
                onChange={(e) => set(key, e.target.value as Address[typeof key])}
                className="mt-1 w-full rounded-lg border border-slate-300 px-3 py-2 text-sm"
              />
            </div>
          ))}

          <div>
            <label htmlFor="country" className="block text-sm text-slate-700">
              Country
            </label>
            <select
              id="country"
              value={address.country}
              onChange={(e) => set("country", e.target.value as Country)}
              className="mt-1 w-full rounded-lg border border-slate-300 px-3 py-2 text-sm"
            >
              {COUNTRIES.map((code) => (
                <option key={code} value={code}>
                  {code}
                </option>
              ))}
            </select>
          </div>

          <div className="flex items-center gap-2">
            <input
              id="billingSame"
              type="checkbox"
              checked={billingSame}
              onChange={(e) => setBillingSame(e.target.checked)}
              className="size-4 rounded border-slate-300"
            />
            <label htmlFor="billingSame" className="text-sm text-slate-700">
              Billing address is the same
            </label>
          </div>

          <button
            type="submit"
            disabled={!complete || pending}
            className="rounded-full bg-slate-900 px-5 py-2.5 text-sm text-white disabled:opacity-40"
          >
            {pending ? "Saving…" : "Continue"}
          </button>
        </form>
      </section>

      <aside className="h-fit rounded-xl border border-slate-200 p-5">
        <h2 className="font-medium text-slate-900">Order summary</h2>
        <ul className="mt-4 space-y-2 text-sm">
          {LINE_ITEMS.map((item) => (
            <li key={item.id} className="flex justify-between text-slate-600">
              <span>{item.name}</span>
              <span className="tabular-nums">{money(item.price)}</span>
            </li>
          ))}
        </ul>
        <dl className="mt-4 space-y-2 border-t border-slate-200 pt-4 text-sm">
          <div className="flex justify-between text-slate-600">
            <dt>Subtotal</dt>
            <dd className="tabular-nums">{money(subtotal)}</dd>
          </div>
          <div className="flex justify-between text-slate-600">
            <dt>Shipping</dt>
            <dd className="tabular-nums">{freeShipping ? "Free" : money(shipping)}</dd>
          </div>
          <div className="flex justify-between font-medium text-slate-900">
            <dt>Total</dt>
            <dd className="tabular-nums">{money(total)}</dd>
          </div>
        </dl>
        <p className="mt-3 text-xs text-slate-500">
          {freeShipping
            ? `Free shipping applied — your order is over ${money(FREE_SHIPPING_OVER)}.`
            : `Add ${money(FREE_SHIPPING_OVER - subtotal + 0.01)} more for free shipping.`}
        </p>
      </aside>
    </main>
  );
}
