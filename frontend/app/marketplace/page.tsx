"use client";

import { useMemo, useState } from "react";
import {
  CreditCard,
  FileText,
  Receipt,
  ShieldCheck,
  ShoppingCart,
  Zap,
  Info,
} from "lucide-react";
import Navbar from "@/components/Navbar";

type Product = {
  id: string;
  name: string;
  tier: string;
  price: number;
  network: string;
  summary: string;
};

const PRODUCTS: Product[] = [
  {
    id: "dex-pro",
    name: "DEX Routing Engine",
    tier: "Commercial",
    price: 249,
    network: "mainnet",
    summary:
      "Multi-hop routing logic with upgrade entitlement and enterprise support.",
  },
  {
    id: "vault-kit",
    name: "Vault Strategy Kit",
    tier: "Team",
    price: 149,
    network: "testnet",
    summary:
      "Managed strategy templates for yield vault contracts and staging deployments.",
  },
  {
    id: "oracle-feed",
    name: "Oracle Feed Adapter",
    tier: "Starter",
    price: 79,
    network: "futurenet",
    summary:
      "Lightweight price feed license with API examples and schema docs.",
  },
];

const TRANSACTIONS = [
  {
    id: "TX-1042",
    item: "DEX Routing Engine",
    amount: "$249",
    status: "Settled",
  },
  {
    id: "TX-1038",
    item: "Vault Strategy Kit",
    amount: "$149",
    status: "Pending",
  },
  {
    id: "TX-1027",
    item: "Oracle Feed Adapter",
    amount: "$79",
    status: "Settled",
  },
];

const STATUS_TONE: Record<string, string> = {
  Settled: "bg-success/10 text-success",
  Pending: "bg-primary/10 text-primary",
};

const LICENSES = [
  { name: "DEX Routing Engine", seats: "12 seats", renews: "2026-01-18" },
  { name: "Oracle Feed Adapter", seats: "3 seats", renews: "2025-11-02" },
];

export default function MarketplacePage() {
  const [cart, setCart] = useState<Product[]>([PRODUCTS[0]]);

  const subtotal = useMemo(
    () => cart.reduce((sum, item) => sum + item.price, 0),
    [cart],
  );

  const addToCart = (product: Product) => {
    setCart((current) => {
      if (current.some((item) => item.id === product.id)) {
        return current;
      }
      return [...current, product];
    });
  };

  const removeFromCart = (productId: string) => {
    setCart((current) => current.filter((item) => item.id !== productId));
  };

  return (
    <div className="min-h-screen bg-background text-foreground">
      <Navbar />

      <section className="relative overflow-hidden border-b border-border ledger-grid">
        <div className="relative max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-16 md:py-20">
          <div className="max-w-3xl">
            <p className="eyebrow inline-flex items-center gap-2 rounded-full border border-border bg-card/60 px-4 py-1.5">
              <span className="h-1.5 w-1.5 rounded-full bg-primary" aria-hidden="true" />
              Marketplace preview
            </p>
            <h1 className="mt-6 text-4xl font-semibold tracking-[-0.03em] sm:text-5xl">
              Contract <span className="text-gradient">marketplace</span>
            </h1>
            <p className="mt-4 max-w-2xl text-lg text-muted-foreground">
              Buy commercial licenses for Soroban contracts, then manage seats
              and renewals in one place.
            </p>
          </div>
        </div>
      </section>

      <main className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-10 space-y-10">
        <div
          role="note"
          className="flex gap-3 rounded-lg border border-primary/40 bg-primary/5 p-4"
        >
          <Info className="mt-0.5 h-5 w-5 shrink-0 text-primary" aria-hidden="true" />
          <div>
            <p className="eyebrow text-primary">Preview, sample data</p>
            <p className="mt-1 text-sm text-foreground">
              This page shows how license sales will work. The products,
              transactions and licenses below are examples, not your account,
              and checkout is turned off until payments launch.
            </p>
          </div>
        </div>

        <section className="grid gap-6 lg:grid-cols-[1.6fr_1fr]">
          <div className="space-y-6">
            <div className="grid gap-5 md:grid-cols-2 2xl:grid-cols-3">
              {PRODUCTS.map((product) => {
                const inCart = cart.some((item) => item.id === product.id);
                return (
                  <article
                    key={product.id}
                    className="gradient-border-card flex flex-col p-6"
                  >
                    <div className="flex items-start justify-between gap-3">
                      <div className="min-w-0">
                        <p className="eyebrow">{product.network}</p>
                        <h2 className="mt-2 text-xl font-semibold">
                          {product.name}
                        </h2>
                      </div>
                      <span className="shrink-0 rounded-full bg-primary/10 px-3 py-1 text-xs font-semibold text-primary">
                        {product.tier}
                      </span>
                    </div>
                    <p className="mt-4 text-sm leading-6 text-muted-foreground">
                      {product.summary}
                    </p>
                    <div className="mt-auto flex items-end justify-between pt-6">
                      <div>
                        <p className="text-xs text-muted-foreground">
                          License price
                        </p>
                        <p className="font-mono text-3xl font-semibold tabular-nums">
                          ${product.price}
                        </p>
                      </div>
                      <button
                        type="button"
                        onClick={() => addToCart(product)}
                        disabled={inCart}
                        className="shrink-0 whitespace-nowrap rounded-md bg-primary px-4 py-2 text-sm font-semibold text-primary-foreground transition-opacity hover:opacity-90 disabled:cursor-default disabled:bg-accent disabled:text-muted-foreground disabled:opacity-100"
                      >
                        {inCart ? "In cart" : "Add to cart"}
                      </button>
                    </div>
                  </article>
                );
              })}
            </div>

            <section className="rounded-lg border border-border bg-card p-6">
              <div className="flex items-center gap-3">
                <Receipt className="h-5 w-5 text-primary" />
                <h2 className="text-xl font-semibold">Example transactions</h2>
              </div>
              <div className="mt-5 space-y-3">
                {TRANSACTIONS.map((transaction) => (
                  <div
                    key={transaction.id}
                    className="flex flex-col gap-3 rounded-md border border-border bg-background p-4 md:flex-row md:items-center md:justify-between"
                  >
                    <div>
                      <p className="font-medium">{transaction.item}</p>
                      <p className="font-mono text-xs text-muted-foreground">
                        {transaction.id}
                      </p>
                    </div>
                    <div className="flex items-center gap-4">
                      <span className="font-mono text-sm font-semibold tabular-nums">
                        {transaction.amount}
                      </span>
                      <span
                        className={`rounded-full px-3 py-1 text-xs font-medium ${STATUS_TONE[transaction.status] ?? "bg-accent text-accent-foreground"}`}
                      >
                        {transaction.status}
                      </span>
                    </div>
                  </div>
                ))}
              </div>
            </section>
          </div>

          <div className="space-y-6">
            <section className="rounded-lg border border-border bg-card p-6">
              <div className="flex items-center gap-3">
                <ShoppingCart className="h-5 w-5 text-primary" />
                <h2 className="text-xl font-semibold">Cart</h2>
              </div>
              <div className="mt-5 space-y-3">
                {cart.length === 0 ? (
                  <p className="rounded-md border border-dashed border-border p-4 text-sm text-muted-foreground">
                    Your cart is empty. Add a license to see the checkout total.
                  </p>
                ) : (
                  cart.map((item) => (
                    <div
                      key={item.id}
                      className="rounded-md border border-border p-4"
                    >
                      <div className="flex items-start justify-between gap-3">
                        <div>
                          <p className="font-medium">{item.name}</p>
                          <p className="text-sm text-muted-foreground">
                            {item.tier} license
                          </p>
                        </div>
                        <button
                          type="button"
                          onClick={() => removeFromCart(item.id)}
                          aria-label={`Remove ${item.name} from cart`}
                          className="text-sm text-muted-foreground hover:text-foreground"
                        >
                          Remove
                        </button>
                      </div>
                      <p className="mt-3 font-mono text-lg font-semibold tabular-nums">
                        ${item.price}
                      </p>
                    </div>
                  ))
                )}
              </div>
            </section>

            <section className="rounded-lg border border-border bg-card p-6">
              <div className="flex items-center gap-3">
                <CreditCard className="h-5 w-5 text-primary" />
                <h2 className="text-xl font-semibold">Checkout</h2>
              </div>
              <div className="mt-5 space-y-4">
                <div className="rounded-md bg-accent p-4 text-sm">
                  <div className="flex items-center justify-between">
                    <span className="text-muted-foreground">Subtotal</span>
                    <span className="font-mono font-semibold tabular-nums">${subtotal}</span>
                  </div>
                  <div className="mt-2 flex items-center justify-between">
                    <span className="text-muted-foreground">Processing</span>
                    <span className="font-mono font-semibold tabular-nums">
                      ${cart.length === 0 ? 0 : 12}
                    </span>
                  </div>
                  <div className="mt-3 border-t border-border pt-3 flex items-center justify-between text-base">
                    <span className="font-semibold">Total</span>
                    <span className="font-mono font-semibold tabular-nums">
                      ${subtotal + (cart.length === 0 ? 0 : 12)}
                    </span>
                  </div>
                </div>
                <button
                  type="button"
                  className="w-full rounded-md bg-primary px-4 py-3 font-semibold text-primary-foreground disabled:cursor-not-allowed disabled:opacity-50"
                  disabled
                >
                  Checkout opens at launch
                </button>
                <p className="text-xs text-muted-foreground">
                  Payments aren’t live yet, so nothing will be charged.
                </p>
              </div>
            </section>

            <section className="rounded-lg border border-border bg-card p-6">
              <div className="flex items-center gap-3">
                <ShieldCheck className="h-5 w-5 text-primary" />
                <h2 className="text-xl font-semibold">Example licenses</h2>
              </div>
              <div className="mt-5 space-y-3">
                {LICENSES.map((license) => (
                  <div
                    key={license.name}
                    className="rounded-md border border-border bg-background p-4"
                  >
                    <div className="flex items-center justify-between gap-3">
                      <p className="font-medium">{license.name}</p>
                      <span className="rounded-full bg-success/10 px-3 py-1 text-xs font-semibold text-success">
                        Active
                      </span>
                    </div>
                    <div className="mt-3 flex items-center justify-between text-sm text-muted-foreground">
                      <span>{license.seats}</span>
                      <span className="font-mono text-xs">Renews {license.renews}</span>
                    </div>
                  </div>
                ))}
              </div>
            </section>
          </div>
        </section>

        <section className="grid gap-5 md:grid-cols-3">
          <div className="rounded-lg border border-border bg-card p-6">
            <Zap className="h-5 w-5 text-primary" />
            <h3 className="mt-4 text-lg font-semibold">Quick purchasing</h3>
            <p className="mt-2 text-sm text-muted-foreground">
              Add licenses to a cart and review the full total before you pay.
            </p>
          </div>
          <div className="rounded-lg border border-border bg-card p-6">
            <FileText className="h-5 w-5 text-primary" />
            <h3 className="mt-4 text-lg font-semibold">License records</h3>
            <p className="mt-2 text-sm text-muted-foreground">
              See the seats and renewal date for every license you hold.
            </p>
          </div>
          <div className="rounded-lg border border-border bg-card p-6">
            <Receipt className="h-5 w-5 text-primary" />
            <h3 className="mt-4 text-lg font-semibold">Transaction tracking</h3>
            <p className="mt-2 text-sm text-muted-foreground">
              Every purchase is recorded with its amount and settlement status.
            </p>
          </div>
        </section>
      </main>
    </div>
  );
}
