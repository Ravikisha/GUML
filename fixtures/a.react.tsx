import { useState } from "react";

export default function CounterCard() {
  const [count, setCount] = useState(0);

  return (
    <div className="mx-auto mt-10 w-full max-w-sm rounded-xl border border-slate-200 bg-white p-6 shadow-sm">
      <h2 className="text-lg font-semibold text-slate-900">Clicks</h2>
      <p className="mt-1 text-sm text-slate-500">Press the buttons to change the value.</p>
      <div className="mt-6 text-center text-5xl font-bold tabular-nums text-slate-900">
        {count}
      </div>
      <div className="mt-6 flex items-center justify-center gap-3">
        <button
          type="button"
          onClick={() => setCount((c) => c - 1)}
          disabled={count === 0}
          className="rounded-md border border-slate-300 px-4 py-2 text-sm font-medium text-slate-700 hover:bg-slate-50 disabled:opacity-40"
        >
          Decrement
        </button>
        <button
          type="button"
          onClick={() => setCount((c) => c + 1)}
          className="rounded-md bg-slate-900 px-4 py-2 text-sm font-medium text-white hover:bg-slate-800"
        >
          Increment
        </button>
        <button
          type="button"
          onClick={() => setCount(0)}
          className="rounded-md px-4 py-2 text-sm font-medium text-slate-500 hover:text-slate-900"
        >
          Reset
        </button>
      </div>
    </div>
  );
}
