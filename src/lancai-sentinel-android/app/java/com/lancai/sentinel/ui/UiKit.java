package com.lancai.sentinel.ui;

// [OWNER: UI] 纯代码 UI 工具箱：卡片、按钮、红绿状态点、对话框、剪贴板。
// 不使用 layout XML / AndroidX / Material，只用平台控件。

import android.app.AlertDialog;
import android.content.ClipData;
import android.content.ClipboardManager;
import android.content.Context;
import android.content.DialogInterface;
import android.content.Intent;
import android.graphics.Color;
import android.graphics.Typeface;
import android.graphics.drawable.GradientDrawable;
import android.text.SpannableString;
import android.text.Spanned;
import android.text.style.ForegroundColorSpan;
import android.util.DisplayMetrics;
import android.view.Gravity;
import android.view.View;
import android.view.ViewGroup;
import android.widget.Button;
import android.widget.LinearLayout;
import android.widget.ScrollView;
import android.widget.TextView;
import android.widget.Toast;

import com.lancai.sentinel.R;
import com.lancai.sentinel.core.RustCore;

import java.text.SimpleDateFormat;
import java.util.Date;
import java.util.Locale;

public final class UiKit {

    public static final int BG = 0xFFF4F6F5;
    public static final int CARD = 0xFFFFFFFF;
    public static final int LINE = 0xFFE3E9E6;
    public static final int TEXT = 0xFF22302C;
    public static final int SUB = 0xFF6B7B76;
    public static final int OK = 0xFF1B7A4B;
    public static final int BAD = 0xFFB3261E;
    public static final int WARN = 0xFFB26A00;

    // v3.6 设计规范：白底、墨色标题、淡紫强调色。
    public static final int INK = 0xFF1B1B20;
    public static final int MUTED = 0xFF7E7E8A;
    public static final int VIOLET = 0xFF5B5BD6;
    public static final int HAIRLINE = 0xFFEFEFF4;

    private UiKit() {
    }

    // ---------------- 尺寸 / 颜色 ----------------
    public static int dp(Context c, float v) {
        try {
            return (int) (v * c.getResources().getDisplayMetrics().density + 0.5f);
        } catch (Throwable t) {
            return (int) v;
        }
    }

    public static int brand(Context c) {
        return c.getResources().getColor(R.color.brand);
    }

    public static int brandDark(Context c) {
        return c.getResources().getColor(R.color.brand_dark);
    }

    public static int accent(Context c) {
        return c.getResources().getColor(R.color.accent);
    }

    public static void rounded(View v, Context c, int fill, int strokeColor, int radiusDp) {
        try {
            GradientDrawable g = new GradientDrawable();
            g.setShape(GradientDrawable.RECTANGLE);
            g.setColor(fill);
            g.setCornerRadius(dp(c, radiusDp));
            if (strokeColor != 0) g.setStroke(Math.max(1, dp(c, 1)), strokeColor);
            v.setBackground(g);
        } catch (Throwable ignore) {
        }
    }

    // ---------------- 容器 ----------------
    public static LinearLayout vertical(Context c) {
        LinearLayout l = new LinearLayout(c);
        l.setOrientation(LinearLayout.VERTICAL);
        return l;
    }

    public static LinearLayout horizontal(Context c) {
        LinearLayout l = new LinearLayout(c);
        l.setOrientation(LinearLayout.HORIZONTAL);
        l.setGravity(Gravity.CENTER_VERTICAL);
        return l;
    }

    /** 白色圆角卡片。 */
    public static LinearLayout card(Context c) {
        LinearLayout l = new LinearLayout(c);
        l.setOrientation(LinearLayout.VERTICAL);
        l.setPadding(dp(c, 14), dp(c, 12), dp(c, 14), dp(c, 12));
        rounded(l, c, CARD, LINE, 12);
        return l;
    }

    /** 可点击的卡片（前景色反馈由 selector 缺省，这里只保证可点）。 */
    public static LinearLayout card(Context c, View.OnClickListener onClick) {
        LinearLayout l = card(c);
        if (onClick != null) l.setOnClickListener(onClick);
        return l;
    }

    public static LinearLayout.LayoutParams lp(int w, int h) {
        return new LinearLayout.LayoutParams(w, h);
    }

    public static LinearLayout.LayoutParams lp(Context c, int wDp, int hDp, int topMarginDp) {
        LinearLayout.LayoutParams p = new LinearLayout.LayoutParams(
                wDp < 0 ? wDp : dp(c, wDp), hDp < 0 ? hDp : dp(c, hDp));
        p.topMargin = dp(c, topMarginDp);
        return p;
    }

    public static LinearLayout.LayoutParams margins(Context c, int l, int t, int r, int b) {
        LinearLayout.LayoutParams p = new LinearLayout.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.WRAP_CONTENT);
        p.setMargins(dp(c, l), dp(c, t), dp(c, r), dp(c, b));
        return p;
    }

    // ---------------- 文本 ----------------
    public static TextView text(Context c, CharSequence s, float sp, int color) {
        TextView t = new TextView(c);
        t.setText(s == null ? "" : s);
        t.setTextSize(sp);
        t.setTextColor(color);
        return t;
    }

    /** 卡片标题（深绿、加粗）。 */
    public static TextView title(Context c, String s) {
        TextView t = text(c, s, 15, brandDark(c));
        t.setTypeface(null, Typeface.BOLD);
        t.setPadding(0, 0, 0, dp(c, 8));
        return t;
    }

    /** 页面分区标题。 */
    public static TextView section(Context c, String s) {
        TextView t = text(c, s, 13, brandDark(c));
        t.setTypeface(null, Typeface.BOLD);
        t.setPadding(0, dp(c, 16), 0, dp(c, 4));
        return t;
    }

    public static TextView h1(Context c, String s) {
        TextView t = text(c, s, 20, brandDark(c));
        t.setTypeface(null, Typeface.BOLD);
        t.setPadding(0, dp(c, 8), 0, dp(c, 4));
        return t;
    }

    public static TextView hint(Context c, String s) {
        TextView t = text(c, s, 12, SUB);
        t.setLineSpacing(dp(c, 3), 1f);
        return t;
    }

    public static TextView label(Context c, String s) {
        TextView t = text(c, s, 12, SUB);
        t.setPadding(0, dp(c, 6), 0, dp(c, 2));
        return t;
    }

    public static TextView body(Context c, String s) {
        TextView t = text(c, s, 12.5f, TEXT);
        t.setLineSpacing(dp(c, 4), 1f);
        return t;
    }

    /** 红绿点 + 状态文字；ok=false 时点是红的。 */
    public static TextView statusLine(Context c, boolean ok, String s) {
        String dot = "● ";
        SpannableString ss = new SpannableString(dot + (s == null ? "" : s));
        ss.setSpan(new ForegroundColorSpan(ok ? OK : BAD), 0, 1,
                Spanned.SPAN_EXCLUSIVE_EXCLUSIVE);
        TextView t = text(c, ss, 13, TEXT);
        t.setPadding(0, dp(c, 4), 0, dp(c, 4));
        return t;
    }

    /** 纯文本分隔说明（橙色，用于"需要手动确认"）。 */
    public static TextView warnLine(Context c, String s) {
        TextView t = text(c, s, 12.5f, WARN);
        t.setLineSpacing(dp(c, 3), 1f);
        return t;
    }

    public static View divider(Context c) {
        View v = new View(c);
        v.setBackgroundColor(LINE);
        v.setLayoutParams(new LinearLayout.LayoutParams(ViewGroup.LayoutParams.MATCH_PARENT, Math.max(1, dp(c, 1))));
        return v;
    }

    // ---------------- 按钮 ----------------
    public static Button button(Context c, String s, int bgColor, View.OnClickListener l) {
        Button b = new Button(c);
        b.setAllCaps(false);
        b.setText(s);
        b.setTextSize(14);
        b.setTextColor(Color.WHITE);
        b.setMinHeight(0);
        b.setMinimumHeight(0);
        b.setPadding(dp(c, 10), 0, dp(c, 10), 0);
        rounded(b, c, bgColor, 0, 10);
        if (android.os.Build.VERSION.SDK_INT >= 21) b.setStateListAnimator(null);
        b.setOnClickListener(l);
        b.setLayoutParams(new LinearLayout.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT, dp(c, 44)));
        return b;
    }

    /** 描边按钮（次要操作）。 */
    public static Button ghostButton(Context c, String s, int textColor, View.OnClickListener l) {
        Button b = new Button(c);
        b.setAllCaps(false);
        b.setText(s);
        b.setTextSize(13);
        b.setTextColor(textColor);
        b.setMinHeight(0);
        b.setMinimumHeight(0);
        b.setPadding(dp(c, 8), 0, dp(c, 8), 0);
        rounded(b, c, 0x00FFFFFF, textColor, 10);
        if (android.os.Build.VERSION.SDK_INT >= 21) b.setStateListAnimator(null);
        b.setOnClickListener(l);
        b.setLayoutParams(new LinearLayout.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT, dp(c, 38)));
        return b;
    }

    /** 小标签（圆角底色 + 白字）。 */
    public static TextView tag(Context c, String s, int bgColor) {
        TextView t = text(c, s, 11, Color.WHITE);
        t.setPadding(dp(c, 7), dp(c, 2), dp(c, 7), dp(c, 3));
        rounded(t, c, bgColor, 0, 8);
        t.setLayoutParams(new LinearLayout.LayoutParams(
                ViewGroup.LayoutParams.WRAP_CONTENT, ViewGroup.LayoutParams.WRAP_CONTENT));
        return t;
    }

    /** 衬线字体：用于页面标题、课程名等「日历感」文字。 */
    public static TextView serif(Context c, CharSequence s, float sp, int color) {
        TextView t = text(c, s, sp, color);
        t.setTypeface(Typeface.SERIF);
        return t;
    }

    /** 浅色顶栏：标题 + 若干图标按钮，白底、无阴影。 */
    public static LinearLayout appBar(Context c, String title,
                                      String[] glyphs, View.OnClickListener[] actions,
                                      View.OnClickListener onTitle) {
        LinearLayout bar = new LinearLayout(c);
        bar.setOrientation(LinearLayout.HORIZONTAL);
        bar.setGravity(Gravity.CENTER_VERTICAL);
        bar.setBackgroundColor(Color.WHITE);
        bar.setPadding(dp(c, 18), dp(c, 6), dp(c, 4), dp(c, 4));

        TextView t = serif(c, title, 22, INK);
        t.setTypeface(Typeface.create(Typeface.SERIF, Typeface.BOLD));
        t.setPadding(0, dp(c, 4), 0, dp(c, 4));
        if (onTitle != null) t.setOnClickListener(onTitle);
        bar.addView(t);
        bar.addView(new View(c), new LinearLayout.LayoutParams(0, 1, 1f));

        if (glyphs != null) {
            for (int i = 0; i < glyphs.length; i++) {
                TextView g = text(c, glyphs[i], 19, 0xFF4E4E5A);
                g.setGravity(Gravity.CENTER);
                g.setPadding(dp(c, 9), dp(c, 6), dp(c, 9), dp(c, 6));
                if (actions != null && i < actions.length && actions[i] != null) {
                    g.setOnClickListener(actions[i]);
                }
                bar.addView(g, new LinearLayout.LayoutParams(
                        ViewGroup.LayoutParams.WRAP_CONTENT,
                        ViewGroup.LayoutParams.WRAP_CONTENT));
            }
        }
        return bar;
    }

    /** 底部弹出面板：从屏幕底部升起，顶部圆角，带拖拽条。 */
    public static AlertDialog sheet(Context c, View content) {
        AlertDialog d = new AlertDialog.Builder(c).setView(content).create();
        try {
            android.view.Window w = d.getWindow();
            if (w != null) {
                w.setBackgroundDrawable(new android.graphics.drawable.ColorDrawable(Color.TRANSPARENT));
                w.setGravity(Gravity.BOTTOM);
                android.view.WindowManager.LayoutParams lp = w.getAttributes();
                lp.width = ViewGroup.LayoutParams.MATCH_PARENT;
                lp.height = ViewGroup.LayoutParams.WRAP_CONTENT;
                w.setAttributes(lp);
            }
        } catch (Throwable ignore) {
        }
        return d;
    }

    /** 面板顶部的拖拽条。 */
    public static View handle(Context c) {
        View v = new View(c);
        v.setBackgroundColor(0xFFE2E2EA);
        LinearLayout.LayoutParams p = new LinearLayout.LayoutParams(dp(c, 36), dp(c, 4));
        p.gravity = Gravity.CENTER_HORIZONTAL;
        p.topMargin = dp(c, 8);
        v.setLayoutParams(p);
        return v;
    }
    public static LinearLayout topBar(final android.app.Activity a, String title) {
        LinearLayout bar = new LinearLayout(a);
        bar.setOrientation(LinearLayout.HORIZONTAL);
        bar.setGravity(Gravity.CENTER_VERTICAL);
        bar.setBackgroundColor(Color.WHITE);
        bar.setPadding(dp(a, 4), dp(a, 6), dp(a, 12), dp(a, 6));

        TextView back = text(a, "‹", 24, INK);
        back.setPadding(dp(a, 12), dp(a, 2), dp(a, 10), dp(a, 6));
        back.setOnClickListener(new View.OnClickListener() {
            @Override
            public void onClick(View v) {
                try {
                    a.finish();
                } catch (Throwable ignore) {
                }
            }
        });
        bar.addView(back, new LinearLayout.LayoutParams(
                ViewGroup.LayoutParams.WRAP_CONTENT, ViewGroup.LayoutParams.WRAP_CONTENT));

        TextView t = serif(a, title, 19, INK);
        t.setTypeface(Typeface.create(Typeface.SERIF, Typeface.BOLD));
        t.setGravity(Gravity.CENTER_VERTICAL | Gravity.START);
        t.setPadding(dp(a, 2), 0, 0, 0);
        bar.addView(t, new LinearLayout.LayoutParams(0,
                ViewGroup.LayoutParams.WRAP_CONTENT, 1f));
        return bar;
    }

    /** 顶部工具条 + 右侧文字按钮（如「保存」）。 */
    public static LinearLayout topBar(final android.app.Activity a, String title,
                                      String rightText, final View.OnClickListener rightAction) {
        LinearLayout bar = topBar(a, title);
        if (rightText != null) {
            TextView right = text(a, rightText, 14, VIOLET);
            right.setTypeface(null, Typeface.BOLD);
            right.setPadding(dp(a, 10), dp(a, 8), dp(a, 6), dp(a, 8));
            right.setOnClickListener(rightAction);
            bar.addView(right, new LinearLayout.LayoutParams(
                    ViewGroup.LayoutParams.WRAP_CONTENT, ViewGroup.LayoutParams.WRAP_CONTENT));
        }
        return bar;
    }

    /** 顶部工具条中间标题（用于挂长按事件）。 */
    public static TextView topBarTitle(LinearLayout bar) {
        try {
            return bar != null && bar.getChildCount() > 1 && bar.getChildAt(1) instanceof TextView
                    ? (TextView) bar.getChildAt(1) : null;
        } catch (Throwable t) {
            return null;
        }
    }

    // ---------------- 底部导航 ----------------
    /** 底部弹出的「功能」面板：日程 / 课表 / 第二课堂 / 设置 + 工具。 */
    public static void menuSheet(final android.app.Activity a) {
        LinearLayout box = vertical(a);
        box.setBackgroundColor(Color.WHITE);
        box.setPadding(dp(a, 16), dp(a, 10), dp(a, 16), dp(a, 18));

        View handle = new View(a);
        handle.setBackgroundColor(0xFFE4E4EC);
        LinearLayout.LayoutParams hlp = new LinearLayout.LayoutParams(
                dp(a, 36), Math.max(1, dp(a, 4)));
        hlp.gravity = Gravity.CENTER_HORIZONTAL;
        hlp.bottomMargin = dp(a, 8);
        box.addView(handle, hlp);

        final AlertDialog d = new AlertDialog.Builder(a).setView(box).create();

        Intent importI = new Intent(a, TimetableActivity.class);
        importI.putExtra("action", "import");
        box.addView(sheetRow(a, "导入课表", "支持 .ics / JSON / CSV 与粘贴文本", d, importI));
        Intent termI = new Intent(a, TimetableActivity.class);
        termI.putExtra("action", "term");
        box.addView(sheetRow(a, "学期设置", "开学日期与总周数", d, termI));

        box.addView(sheetRow(a, "日程与待办", "新建、完成、提醒与自动报名", d,
                new Intent(a, ScheduleActivity.class)));
        box.addView(sheetRow(a, "完整课表", "导入、编辑、切换周次", d,
                new Intent(a, TimetableActivity.class)));
        box.addView(sheetRow(a, "第二课堂", "活动浏览与记到课表", d,
                new Intent(a, DektActivity.class)));
        box.addView(sheetRow(a, "设置", "分析规则、AI、推送与通知", d,
                new Intent(a, SettingsActivity.class)));
        box.addView(divider(a));
        box.addView(sheetAction(a, "规则自检", "检查内核、权限与配置", d,
                new Runnable() {
                    @Override
                    public void run() {
                        infoDialog(a, "规则自检", RustCore.selfTest(a));
                    }
                }));
        box.addView(sheetAction(a, "清空最近记录", "删除本地消息库与摘要", d,
                new Runnable() {
                    @Override
                    public void run() {
                        confirm(a, "清空最近记录", "会删除本地消息库和最近摘要，不可恢复。",
                                "清空", new Runnable() {
                                    @Override
                                    public void run() {
                                        boolean ok = RustCore.clear(a);
                                        toast(a, ok ? "已清空" : "清空失败");
                                    }
                                });
                    }
                }));
        box.addView(sheetAction(a, "关于", "版本与使用说明", d,
                new Runnable() {
                    @Override
                    public void run() {
                        infoDialog(a, "关于澜彩哨兵",
                                "澜彩哨兵 3.7\n\n"
                                        + "· 课表、日程、第二课堂集中在一处\n"
                                        + "· 活动识别与报名判断在本地 Rust 内核完成\n"
                                        + "· 所有数据只保存在本机，不上传服务器\n"
                                        + "· 自动报名需逐条确认，到点提醒后再打开报名页");
                    }
                }));

        d.show();
        try {
            android.view.Window w = d.getWindow();
            if (w != null) {
                w.setBackgroundDrawable(
                        new android.graphics.drawable.ColorDrawable(Color.TRANSPARENT));
                w.setGravity(Gravity.BOTTOM);
                android.view.WindowManager.LayoutParams p = w.getAttributes();
                p.width = android.view.WindowManager.LayoutParams.MATCH_PARENT;
                w.setAttributes(p);
            }
        } catch (Throwable ignore) {
        }
    }

    private static View sheetRow(final Context a, String title, String desc,
                                 final AlertDialog d, final Intent intent) {
        LinearLayout row = horizontal(a);
        row.setPadding(dp(a, 4), dp(a, 13), dp(a, 4), dp(a, 13));
        LinearLayout col = vertical(a);
        TextView t = text(a, title, 14.5f, TEXT);
        t.setTypeface(null, Typeface.BOLD);
        col.addView(t);
        col.addView(text(a, desc, 11.5f, SUB));
        row.addView(col, new LinearLayout.LayoutParams(0,
                ViewGroup.LayoutParams.WRAP_CONTENT, 1f));
        row.addView(text(a, "›", 20, SUB));
        row.setOnClickListener(new View.OnClickListener() {
            @Override
            public void onClick(View v) {
                try {
                    d.dismiss();
                    a.startActivity(intent);
                } catch (Throwable ignore) {
                }
            }
        });
        return row;
    }

    private static View sheetAction(final Context a, String title, String desc,
                                    final AlertDialog d, final Runnable action) {
        LinearLayout row = horizontal(a);
        row.setPadding(dp(a, 4), dp(a, 13), dp(a, 4), dp(a, 13));
        LinearLayout col = vertical(a);
        col.addView(text(a, title, 14.5f, TEXT));
        col.addView(text(a, desc, 11.5f, SUB));
        row.addView(col, new LinearLayout.LayoutParams(0,
                ViewGroup.LayoutParams.WRAP_CONTENT, 1f));
        row.setOnClickListener(new View.OnClickListener() {
            @Override
            public void onClick(View v) {
                d.dismiss();
                action.run();
            }
        });
        return row;
    }

    /** 白色底部导航：日程 / 课表 / 第二课堂 / 菜单 / 我的。active 传当前页 key。 */
    public static LinearLayout bottomNav(final android.app.Activity a, String active) {
        LinearLayout wrap = vertical(a);
        wrap.setBackgroundColor(Color.WHITE);
        View line = new View(a);
        line.setBackgroundColor(LINE);
        wrap.addView(line, new LinearLayout.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT, Math.max(1, dp(a, 1))));

        LinearLayout row = horizontal(a);
        row.setPadding(dp(a, 4), dp(a, 7), dp(a, 4), dp(a, 9));
        navItem(a, row, "▦", "日程", "schedule", active);
        navItem(a, row, "▤", "课表", "timetable", active);
        navItem(a, row, "⌂", "第二课堂", "dekt", active);
        navItem(a, row, "≡", "菜单", "menu", active);
        navItem(a, row, "☺", "我的", "me", active);
        wrap.addView(row);
        return wrap;
    }

    private static void navItem(final android.app.Activity a, LinearLayout row,
                                String glyph, String label, final String key, String active) {
        final boolean sel = key.equals(active);
        LinearLayout item = vertical(a);
        item.setGravity(Gravity.CENTER);
        TextView icon = text(a, glyph, 18, sel ? 0xFF5B5BD6 : 0xFF9A9AA6);
        icon.setGravity(Gravity.CENTER);
        item.addView(icon);
        TextView labelView = text(a, label, 10, sel ? 0xFF5B5BD6 : 0xFF7E7E8A);
        labelView.setGravity(Gravity.CENTER);
        labelView.setPadding(0, dp(a, 1), 0, 0);
        item.addView(labelView);
        item.setOnClickListener(new View.OnClickListener() {
            @Override
            public void onClick(View v) {
                Intent i;
                if ("schedule".equals(key)) {
                    i = new Intent(a, ScheduleActivity.class);
                } else if ("timetable".equals(key)) {
                    i = new Intent(a, TimetableActivity.class);
                } else if ("dekt".equals(key)) {
                    i = new Intent(a, DektActivity.class);
                } else if ("menu".equals(key)) {
                    menuSheet(a);
                    return;
                } else {
                    i = new Intent(a, SettingsActivity.class);
                }
                try {
                    if (i.getComponent() != null
                            && i.getComponent().getClassName().equals(a.getClass().getName())) {
                        return;
                    }
                    a.startActivity(i);
                } catch (Throwable ignore) {
                }
            }
        });
        row.addView(item, new LinearLayout.LayoutParams(0,
                ViewGroup.LayoutParams.WRAP_CONTENT, 1f));
    }

    // ---------------- 交互 ----------------
    public static void toast(Context c, String s) {
        if (c == null || s == null) return;
        try {
            Toast.makeText(c, s, Toast.LENGTH_SHORT).show();
        } catch (Throwable ignore) {
        }
    }

    public static void toastLong(Context c, String s) {
        if (c == null || s == null) return;
        try {
            Toast.makeText(c, s, Toast.LENGTH_LONG).show();
        } catch (Throwable ignore) {
        }
    }

    public static void copy(Context c, String label, String text) {
        if (c == null) return;
        try {
            ClipboardManager cm = (ClipboardManager) c.getSystemService(Context.CLIPBOARD_SERVICE);
            if (cm == null) return;
            cm.setPrimaryClip(ClipData.newPlainText(label == null ? "澜彩哨兵" : label,
                    text == null ? "" : text));
            toast(c, "已复制到剪贴板");
        } catch (Throwable t) {
            toast(c, "复制失败");
        }
    }

    private static ScrollView scrollBody(Context c, String body) {
        ScrollView sc = new ScrollView(c);
        TextView t = text(c, body, 13, TEXT);
        t.setTextIsSelectable(true);
        t.setLineSpacing(dp(c, 4), 1f);
        t.setPadding(dp(c, 16), dp(c, 6), dp(c, 16), dp(c, 6));
        sc.addView(t);
        int h = 420;
        try {
            DisplayMetrics dm = c.getResources().getDisplayMetrics();
            h = (int) (dm.heightPixels / dm.density * 0.62f);
        } catch (Throwable ignore) {
        }
        sc.setLayoutParams(new ViewGroup.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT, Math.max(dp(c, 200), dp(c, h))));
        return sc;
    }

    /** 长文本弹窗（可选中复制）。 */
    public static void infoDialog(Context c, String title, String bodyText) {
        infoDialog(c, title, bodyText, null, null);
    }

    /** 长文本弹窗 + 一个中立操作（如"清空记录"）。 */
    public static void infoDialog(Context c, String title, String bodyText,
                                  String neutral, final Runnable neutralAction) {
        if (c == null) return;
        try {
            AlertDialog.Builder b = new AlertDialog.Builder(c);
            b.setTitle(title);
            b.setView(scrollBody(c, bodyText == null ? "" : bodyText));
            b.setPositiveButton("关闭", null);
            if (neutral != null && neutralAction != null) {
                b.setNeutralButton(neutral, new DialogInterface.OnClickListener() {
                    @Override
                    public void onClick(DialogInterface d, int w) {
                        try {
                            neutralAction.run();
                        } catch (Throwable ignore) {
                        }
                    }
                });
            }
            AlertDialog d = b.create();
            d.show();
        } catch (Throwable t) {
            toast(c, "打开对话框失败");
        }
    }

    public static void confirm(Context c, String title, String bodyText,
                               String okText, final Runnable okAction) {
        if (c == null) return;
        try {
            AlertDialog.Builder b = new AlertDialog.Builder(c);
            b.setTitle(title);
            b.setMessage(bodyText == null ? "" : bodyText);
            b.setPositiveButton(okText == null ? "确定" : okText, new DialogInterface.OnClickListener() {
                @Override
                public void onClick(DialogInterface d, int w) {
                    if (okAction != null) {
                        try {
                            okAction.run();
                        } catch (Throwable ignore) {
                        }
                    }
                }
            });
            b.setNegativeButton("取消", null);
            b.create().show();
        } catch (Throwable t) {
            toast(c, "打开对话框失败");
        }
    }

    // ---------------- 时间格式 ----------------
    public static String hhmm(long ms) {
        return format(ms, "HH:mm");
    }

    public static String hhmmss(long ms) {
        return format(ms, "HH:mm:ss");
    }

    public static String mdhm(long ms) {
        return format(ms, "MM-dd HH:mm");
    }

    public static String ymdhm(long ms) {
        return format(ms, "yyyy-MM-dd HH:mm");
    }

    private static String format(long ms, String pattern) {
        try {
            return new SimpleDateFormat(pattern, Locale.CHINA).format(new Date(ms));
        } catch (Throwable t) {
            return "";
        }
    }

    /** "刚刚 / 3 分钟前 / 2 小时前 / MM-dd HH:mm"。 */
    public static String ago(long ms) {
        if (ms <= 0) return "暂无";
        long d = System.currentTimeMillis() - ms;
        if (d < 0) d = 0;
        if (d < 60_000L) return "刚刚";
        if (d < 3_600_000L) return (d / 60_000L) + " 分钟前";
        if (d < 24 * 3_600_000L) return (d / 3_600_000L) + " 小时前";
        return mdhm(ms);
    }

    /** 时长："2 小时 5 分"。 */
    public static String duration(long ms) {
        if (ms < 0) ms = 0;
        long min = ms / 60_000L;
        if (min < 1) return "刚刚";
        long h = min / 60;
        long m = min % 60;
        if (h <= 0) return m + " 分钟";
        if (h > 24) return (h / 24) + " 天 " + (h % 24) + " 小时";
        return h + " 小时 " + m + " 分";
    }
}
