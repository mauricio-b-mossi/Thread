<script lang="ts">
  type EntryRoute = "today" | "floating";

  const getRoute = (): EntryRoute => {
    const route = window.location.hash.replace(/^#\/?/, "") || window.location.pathname.replace(/^\//, "");
    return route === "floating" ? "floating" : "today";
  };

  let route = $state<EntryRoute>(getRoute());
  const today = new Date();
  const todayIso = [
    today.getFullYear(),
    String(today.getMonth() + 1).padStart(2, "0"),
    String(today.getDate()).padStart(2, "0")
  ].join("-");
  const todayLabel = new Intl.DateTimeFormat(undefined, {
    month: "short",
    day: "numeric"
  }).format(today);

  const updateRoute = () => {
    route = getRoute();
  };
</script>

<svelte:window onhashchange={updateRoute} onpopstate={updateRoute} />

{#if route === "floating"}
  <main class="floating-shell" aria-label="Thread floating window">
    <div class="floating-bar">
      <span class="status-dot"></span>
      <span>Thread</span>
    </div>
    <p>Floating capture placeholder</p>
  </main>
{:else}
  <main class="today-shell" aria-label="Thread Today">
    <section class="topbar">
      <div>
        <p class="eyebrow">Thread</p>
        <h1>Today</h1>
      </div>
      <time datetime={todayIso}>{todayLabel}</time>
    </section>

    <section class="focus-panel" aria-labelledby="focus-heading">
      <div>
        <p class="eyebrow">Current thread</p>
        <h2 id="focus-heading">Start with the next useful note.</h2>
      </div>
      <button type="button">New note</button>
    </section>

    <section class="list-panel" aria-label="Today queue">
      <article>
        <span class="item-marker"></span>
        <div>
          <h3>Daily planning</h3>
          <p>Inbox, priorities, and quick notes for the day.</p>
        </div>
      </article>
      <article>
        <span class="item-marker muted"></span>
        <div>
          <h3>Floating window</h3>
          <p>Capture surface placeholder for the compact companion window.</p>
        </div>
      </article>
    </section>
  </main>
{/if}
