<script setup lang="ts">
import { ref, onMounted, onBeforeUnmount } from "vue";
import { useRouter } from "vue-router";
import { Play, ImageOff, Star } from "lucide-vue-next";
import { imgUrl } from "../../lib/api";
import * as api from "../../lib/api";
import type { DlsiteRankingItem } from "../../lib/types";

const router = useRouter();
const picks = ref<DlsiteRankingItem[]>([]);
const cur = ref(0);
let timer: number | null = null;

const titleOf = (w: DlsiteRankingItem) => w.title || w.rj_code;

onMounted(async () => {
  try {
    const res = await api.getDlsiteRanking("day", 30);
    if (!res.items.length) return;
    // 从日榜 Top30 随机抽 3 部，每次打开页面都不同（与设计稿逻辑一致）
    const pool = [...res.items];
    for (let i = pool.length - 1; i > 0; i--) {
      const j = Math.floor(Math.random() * (i + 1));
      [pool[i], pool[j]] = [pool[j], pool[i]];
    }
    picks.value = pool.slice(0, 3);
    play();
  } catch {
    // 榜单不可用时静默隐藏轮播，不影响下方作品列表
  }
});

function play() {
  stop();
  timer = window.setInterval(() => go(cur.value + 1), 4600);
}
function stop() {
  if (timer) {
    window.clearInterval(timer);
    timer = null;
  }
}
function go(i: number) {
  cur.value = (i + picks.value.length) % picks.value.length;
}
function openSlide(w: DlsiteRankingItem) {
  if (w.work_id) router.push(`/work/${w.work_id}`);
  else router.push(`/dlsite/work/${w.rj_code}`);
}
onBeforeUnmount(stop);
</script>

<template>
  <section v-if="picks.length" class="carousel">
    <div class="carousel-stage">
      <article
        v-for="(w, i) in picks"
        :key="w.rj_code"
        class="slide"
        :class="{ active: i === cur }"
      >
        <!-- 背景：基于封面的大图虚化 -->
        <div
          v-if="w.cover_url"
          class="slide-bg"
          :style="{ backgroundImage: `url(${imgUrl(w.cover_url)})` }"
        />
        <!-- 左侧内嵌封面：原始 4:3 圆角卡片 -->
        <div class="slide-pic">
          <img v-if="w.cover_url" :src="imgUrl(w.cover_url)" :alt="titleOf(w)" />
          <div v-else class="pic-fallback">
            <ImageOff :size="26" />
          </div>
        </div>

        <div class="slide-body">
          <div class="slide-main">
            <span class="eyebrow">日榜 No.{{ w.rank }} · TOP CHART</span>
            <h2 :title="titleOf(w)">{{ titleOf(w) }}</h2>
            <p class="meta">
              <span v-if="w.rating != null" class="meta-rating" title="DLsite 评分">
                <Star :size="12" class="meta-star" /> {{ w.rating }} ·
              </span>售出
              <span class="meta-strong">{{ (w.dl_count ?? 0).toLocaleString() }}</span>
            </p>
            <div class="slide-info">
              <div class="info-item">
                <span class="info-k">RJ</span>
                <span class="info-v font-mono">{{ w.rj_code }}</span>
              </div>
              <div v-if="w.sale_date" class="info-item">
                <span class="info-k">发售</span>
                <span class="info-v">{{ w.sale_date }}</span>
              </div>
              <div v-if="w.circle_name" class="info-item">
                <span class="info-k">社团</span>
                <span class="info-v">{{ w.circle_name }}</span>
              </div>
              <div v-if="w.vas.length" class="info-item">
                <span class="info-k">声优</span>
                <span class="info-v">{{ w.vas.join(" / ") }}</span>
              </div>
              <div v-if="w.tags.length" class="slide-tags">
                <span v-for="t in w.tags.slice(0, 4)" :key="t" class="tag">{{ t }}</span>
              </div>
            </div>
          </div>
        </div>

        <button class="slide-cta" @click="openSlide(w)">
          <Play :size="13" /> 立即播放
        </button>
      </article>
    </div>

    <div class="carousel-dots">
      <button
        v-for="(w, i) in picks"
        :key="w.rj_code"
        :class="{ on: i === cur }"
        :aria-label="`第 ${i + 1} 张`"
        @click="go(i); play()"
      />
    </div>
  </section>
</template>

<style scoped>
/* 宽度与内边距由页面容器（.page-wrap）统一控制，避免双层 padding 把舞台收窄 */
.carousel {
  margin: 0;
}
.carousel-stage {
  position: relative;
  border-radius: var(--r-xl);
  overflow: hidden;
  aspect-ratio: 10 / 3;
  box-shadow: var(--shadow-card), inset 0 0 0 0.5px rgba(255, 255, 255, 0.08);
  background: var(--bg-secondary);
}
.slide {
  position: absolute;
  inset: 0;
  display: flex;
  align-items: center;
  gap: 36px;
  padding: 40px;
  color: #fff;
  opacity: 0;
  transition: opacity 0.7s cubic-bezier(0.3, 0.6, 0.2, 1);
  pointer-events: none;
}
.slide.active {
  opacity: 1;
  pointer-events: auto;
}
/* 背景:基于封面图的大图虚化,同图不同帧避免缓存穿透 */
.slide-bg {
  position: absolute;
  inset: -40px;
  background-size: cover;
  background-position: center 30%;
  filter: blur(34px) saturate(1.35) brightness(0.55);
  transform: scale(1.12);
}
.slide-bg::after {
  content: "";
  position: absolute;
  inset: 0;
  background:
    linear-gradient(100deg, rgba(8, 8, 14, 0.62) 0%, rgba(8, 8, 14, 0.34) 55%, rgba(8, 8, 14, 0.2) 100%),
    radial-gradient(90% 130% at 50% -20%, rgba(255, 255, 255, 0.1), transparent 60%);
}
/* 柔光球装饰 */
.slide::before {
  content: "";
  position: absolute;
  top: -30%;
  left: 38%;
  width: 36%;
  aspect-ratio: 1;
  border-radius: 50%;
  background: radial-gradient(circle at 40% 36%, rgba(255, 255, 255, 0.13), rgba(255, 255, 255, 0.04) 58%, transparent 72%);
  pointer-events: none;
}
/* 左侧内嵌封面:原始 4:3,圆角卡片 */
.slide-pic {
  position: relative;
  z-index: 1;
  height: 100%;
  aspect-ratio: 4 / 3;
  align-self: center;
  flex-shrink: 0;
  border-radius: 18px;
  overflow: hidden;
  box-shadow: 0 18px 44px -14px rgba(0, 0, 0, 0.6), inset 0 0 0 0.5px rgba(255, 255, 255, 0.18);
  background: var(--fill-1);
}
.slide-pic img {
  width: 100%;
  height: 100%;
  object-fit: cover;
  display: block;
}
.pic-fallback {
  width: 100%;
  height: 100%;
  display: flex;
  align-items: center;
  justify-content: center;
  color: var(--label-3);
}
.slide-body {
  position: relative;
  flex: 1;
  min-width: 0;
  z-index: 1;
  display: flex;
  align-items: center;
  gap: 28px;
}
.slide-main {
  flex: 1;
  min-width: 0;
}
.slide .eyebrow {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  font-size: 12px;
  font-weight: 600;
  letter-spacing: 0.08em;
  background: rgba(255, 255, 255, 0.2);
  backdrop-filter: blur(10px);
  border-radius: 999px;
  padding: 5px 14px;
}
.slide h2 {
  font-size: clamp(16px, 1.7vw, 20px);
  font-weight: 700;
  letter-spacing: -0.02em;
  line-height: 1.35;
  margin: 12px 0 0;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  max-width: 100%;
  color: #fff;
}
.slide .meta {
  font-size: 14px;
  opacity: 0.85;
  margin-top: 7px;
}
/* 评分段：金色实心星标，与榜单卡片 .rc-rating 同风格 */
.meta-rating {
  display: inline-flex;
  align-items: center;
  gap: 3px;
  /* 评分片段自带「·」分隔符，与后续「销售」文字拉开固定间距 */
  margin-right: 4px;
  font-weight: 600;
  color: #fff;
  opacity: 1;
  font-variant-numeric: tabular-nums;
}
.meta-star {
  color: #ffd60a;
  fill: #ffd60a;
}
/* 售出数字：与评分段同字重，纯白不透明 */
.meta-strong {
  font-weight: 600;
  opacity: 1;
  color: #fff;
  font-variant-numeric: tabular-nums;
}
.slide .slide-info {
  margin-top: 12px;
  display: grid;
  /* 第一列（RJ / 社团）固定宽度，保证「发售」「声优」列始终垂直对齐 */
  grid-template-columns: 170px auto;
  justify-content: start;
  column-gap: 56px;
  row-gap: 5px;
}
.slide .info-item {
  display: inline-flex;
  align-items: baseline;
  gap: 6px;
  min-width: 0;
}
.slide .info-k {
  font-size: 11.5px;
  font-weight: 600;
  letter-spacing: 0.08em;
  color: rgba(255, 255, 255, 0.5);
  flex-shrink: 0;
  width: 30px;
}
.slide .info-v {
  font-size: 13px;
  color: rgba(255, 255, 255, 0.85);
}
.slide .slide-tags {
  grid-column: 1 / -1;
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
  margin-top: 4px;
}
.slide .slide-tags .tag {
  font-size: 11px;
  font-weight: 500;
  line-height: 1;
  color: rgba(255, 255, 255, 0.75);
  background: rgba(255, 255, 255, 0.14);
  border: 0.5px solid rgba(255, 255, 255, 0.18);
  border-radius: 999px;
  padding: 5px 11px;
  backdrop-filter: blur(6px);
}
.slide .slide-cta {
  position: absolute;
  right: 40px;
  bottom: 40px;
  z-index: 1;
  display: inline-flex;
  align-items: center;
  gap: 8px;
  height: 40px;
  padding: 0 22px;
  border: none;
  border-radius: 999px;
  background: rgba(255, 255, 255, 0.92);
  color: #1d1d1f;
  font-family: inherit;
  font-size: 13.5px;
  font-weight: 600;
  cursor: pointer;
  transition: transform 0.2s, background 0.2s;
}
.slide .slide-cta:hover {
  transform: scale(1.03);
  background: #fff;
}
.carousel-dots {
  display: flex;
  justify-content: center;
  gap: 8px;
  /* 圆点居中于轮播底边与搜索行之间的空隙（上下等距） */
  margin-top: 11px;
  margin-bottom: 11px;
}
.carousel-dots button {
  width: 8px;
  height: 8px;
  border-radius: 999px;
  background: var(--fill-3);
  border: none;
  padding: 0;
  cursor: pointer;
  transition: all 0.35s cubic-bezier(0.3, 0.6, 0.2, 1);
}
.carousel-dots button:hover {
  background: var(--label-3);
}
.carousel-dots button.on {
  width: 26px;
  background: var(--label);
}

@media (max-width: 900px) {
  .carousel-stage {
    aspect-ratio: 16 / 10;
  }
  .slide {
    padding: 16px;
    gap: 16px;
  }
  .slide-pic {
    border-radius: 12px;
  }
  .slide h2 {
    font-size: 16px;
    margin-top: 9px;
  }
  .slide .meta {
    font-size: 11px;
  }
  .slide .slide-cta {
    height: 32px;
    padding: 0 15px;
    font-size: 12px;
    right: 16px;
    bottom: 16px;
  }
}
</style>
