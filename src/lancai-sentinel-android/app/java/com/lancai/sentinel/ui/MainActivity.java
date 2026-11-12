package com.lancai.sentinel.ui;

// [OWNER: UI] 主页 = 课表（与 TimetableActivity 共用网格，同时显示课程与日程/活动）。
// 分析功能收进「分析」弹窗；识别到活动/待办时自动加入日程，也可手动点「加入日程」。

import android.app.Activity;
import android.app.AlertDialog;
import android.content.Context;
import android.content.DialogInterface;
import android.content.Intent;
import android.graphics.Color;
import android.graphics.Typeface;
import android.graphics.drawable.GradientDrawable;
import android.net.Uri;
import android.os.Bundle;
import android.os.Handler;
import android.os.Looper;
import android.text.TextUtils;
import android.view.Gravity;
import android.view.MotionEvent;
import android.view.View;
import android.view.ViewGroup;
import android.widget.Button;
import android.widget.EditText;
import android.widget.FrameLayout;
import android.widget.HorizontalScrollView;
import android.widget.LinearLayout;
import android.widget.RelativeLayout;
import android.widget.ScrollView;
import android.widget.TextView;

import com.lancai.sentinel.core.Config;
import com.lancai.sentinel.core.NotificationHelper;
import com.lancai.sentinel.core.ReminderScheduler;
import com.lancai.sentinel.core.RustCore;

import org.json.JSONArray;
import org.json.JSONObject;

import java.text.SimpleDateFormat;
import java.util.Calendar;
import java.util.Date;
import java.util.Locale;
import java.util.regex.Matcher;
import java.util.regex.Pattern;

public class MainActivity extends Activity {

    private static final Pattern URL_RE = Pattern.compile("https?://\\S+", Pattern.CASE_INSENSITIVE);

    private final Handler ui = new Handler(Looper.getMainLooper());
    private LinearLayout gridBox;
    private TextView weekLabel, backWeek;
    private String lastTtJson, lastSchJson;
    private int lastRenderWeek = -1;
    private long lastLoadAtMs;
    private WeekSwipe weekSwipe;
    private JSONObject data;
    private JSONArray schedule;
    private int displayWeek;
    private int currentWeek = 1;

    // 分析弹窗
    private AlertDialog analyzeDialog;
    private EditText dlgSource, dlgText;
    private Button dlgAnalyze;
    private LinearLayout resultBox, recentBox;
    private String lastCard = "";
    private Button addScheduleBtn;

    @Override
    protected void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);
        render();
        NotificationHelper.ensureChannel(this);
        NotificationHelper.requestPermission(this);
        handleIntent(getIntent());
    }

    @Override
    protected void onNewIntent(Intent intent) {
        super.onNewIntent(intent);
        setIntent(intent);
        handleIntent(intent);
    }

    @Override
    protected void onResume() {
        super.onResume();
        load();
        // 不再每次 onResume 都全量重排提醒。
        // AlarmManager.setAndAllowWhileIdle 的闹钟会跨进程存活，
        // 只有开机/升级/日程变更才需要 reschedule（已在 BootReceiver / 各保存操作里触发）。
    }

    private void render() {
        RelativeLayout shell = new RelativeLayout(this);
        shell.setBackgroundColor(Color.WHITE);

        View head = header();
        head.setId(0x1001);
        RelativeLayout.LayoutParams hlp = new RelativeLayout.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.WRAP_CONTENT);
        hlp.addRule(RelativeLayout.ALIGN_PARENT_TOP);
        shell.addView(head, hlp);

        View hline = new View(this);
        hline.setBackgroundColor(UiKit.HAIRLINE);
        hline.setId(0x1003);
        RelativeLayout.LayoutParams lineLp = new RelativeLayout.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT, Math.max(1, dp(1)));
        lineLp.addRule(RelativeLayout.ALIGN_PARENT_TOP);
        lineLp.addRule(RelativeLayout.BELOW, 0x1001);
        shell.addView(hline, lineLp);

        ScrollView scroll = new ScrollView(this);
        scroll.setFillViewport(true);
        scroll.setClipToPadding(false);
        scroll.setVerticalScrollBarEnabled(false);

        LinearLayout root = UiKit.vertical(this);
        root.setPadding(0, 0, 0, dp(12));

        LinearLayout card = UiKit.vertical(this);
        GradientDrawable g = new GradientDrawable();
        g.setColor(Color.WHITE);
        g.setCornerRadius(dp(20));
        g.setStroke(Math.max(1, dp(1)), 0xFFF0F0F5);
        card.setBackground(g);
        card.setPadding(dp(4), dp(8), dp(4), dp(8));
        if (android.os.Build.VERSION.SDK_INT >= 21) card.setElevation(dp(2));
        gridBox = UiKit.vertical(this);
        gridBox.addView(UiKit.hint(this, "正在载入课表…"));
        card.addView(gridBox, new LinearLayout.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.WRAP_CONTENT));
        LinearLayout.LayoutParams cardLp = new LinearLayout.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.WRAP_CONTENT);
        cardLp.setMargins(dp(10), dp(2), dp(10), dp(8));
        root.addView(card, cardLp);

        if (!RustCore.available()) {
            root.addView(UiKit.warnLine(this,
                    "Rust 核心未加载，分析/日程不可用。请确认安装包包含本机 ABI 的 liblancai_sentinel.so。"),
                    new LinearLayout.LayoutParams(
                            ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.WRAP_CONTENT));
        }

        scroll.addView(root, new ViewGroup.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.WRAP_CONTENT));

        RelativeLayout.LayoutParams slp = new RelativeLayout.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.MATCH_PARENT);
        slp.addRule(RelativeLayout.BELOW, 0x1003);
        slp.addRule(RelativeLayout.ABOVE, 0x1002);
        shell.addView(scroll, slp);

        LinearLayout nav = UiKit.bottomNav(this, "timetable");
        nav.setId(0x1002);
        RelativeLayout.LayoutParams nlp = new RelativeLayout.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.WRAP_CONTENT);
        nlp.addRule(RelativeLayout.ALIGN_PARENT_BOTTOM);
        shell.addView(nav, nlp);

        backWeek = UiKit.text(this, "↩ 返回本周", 12.5f, Color.WHITE);
        backWeek.setGravity(Gravity.CENTER);
        backWeek.setPadding(dp(14), dp(8), dp(14), dp(9));
        backWeek.setTypeface(null, Typeface.BOLD);
        UiKit.rounded(backWeek, this, 0xFF5B5BD6, 0, 18);
        backWeek.setVisibility(View.GONE);
        backWeek.setOnClickListener(new View.OnClickListener() {
            @Override
            public void onClick(View v) {
                if (data == null) return;
                displayWeek = currentWeek;
                renderWeek();
            }
        });
        RelativeLayout.LayoutParams blp = new RelativeLayout.LayoutParams(
                ViewGroup.LayoutParams.WRAP_CONTENT, ViewGroup.LayoutParams.WRAP_CONTENT);
        blp.addRule(RelativeLayout.ALIGN_PARENT_BOTTOM);
        blp.addRule(RelativeLayout.ALIGN_PARENT_RIGHT);
        blp.setMargins(0, 0, dp(16), dp(74));
        shell.addView(backWeek, blp);

        weekSwipe = new WeekSwipe(new WeekSwipe.Host() {
            @Override
            public View weekView() {
                return gridBox;
            }

            @Override
            public boolean canSwitch() {
                return data != null;
            }

            @Override
            public void switchWeek(int delta, int outDir) {
                animateWeek(delta, outDir);
            }
        });

        setContentView(shell);
    }

    @Override
    public boolean dispatchTouchEvent(MotionEvent ev) {
        if (weekSwipe != null && weekSwipe.onTouch(ev)) return true;
        return super.dispatchTouchEvent(ev);
    }

    /** 拖动松手后：旧课表滑出，新课表从另一侧滑入。 */
    private void animateWeek(int delta, int outDir) {
        if (data == null || gridBox == null) return;
        int total = Math.max(1, data.optInt("weeks", 20));
        final int next = Math.max(1, Math.min(displayWeek + delta, total));
        final int w = Math.max(1, gridBox.getWidth());
        if (next == displayWeek) {
            gridBox.animate().translationX(0f).setDuration(140).start();
            return;
        }
        displayWeek = next;
        gridBox.animate().translationX(outDir * w).setDuration(130)
                .withEndAction(new Runnable() {
                    @Override
                    public void run() {
                        renderWeek();
                        gridBox.setTranslationX(-outDir * w);
                        gridBox.animate().translationX(0f).setDuration(160).start();
                    }
                }).start();
    }

    private View header() {
        LinearLayout head = UiKit.vertical(this);
        head.setBackgroundColor(Color.WHITE);
        head.setPadding(0, dp(4), 0, 0);

        LinearLayout badges = UiKit.horizontal(this);
        weekLabel = UiKit.text(this, "第 1 周", 11.5f, 0xFF5B5BD6);
        weekLabel.setTypeface(null, Typeface.BOLD);
        weekLabel.setGravity(Gravity.CENTER);
        weekLabel.setSingleLine(true);
        weekLabel.setEllipsize(TextUtils.TruncateAt.END);
        weekLabel.setPadding(dp(9), dp(4), dp(9), dp(4));
        UiKit.rounded(weekLabel, this, 0xFFEAEAFB, 0, 10);
        weekLabel.setOnClickListener(new View.OnClickListener() {
            @Override
            public void onClick(View v) {
                showWeekPicker();
            }
        });
        badges.addView(weekLabel);

        TextView today = UiKit.text(this, "今天", 11.5f, 0xFF6E6E7A);
        today.setGravity(Gravity.CENTER);
        today.setPadding(dp(9), dp(4), dp(9), dp(4));
        UiKit.rounded(today, this, 0xFFF2F2F7, 0, 10);
        LinearLayout.LayoutParams tlp = new LinearLayout.LayoutParams(
                ViewGroup.LayoutParams.WRAP_CONTENT, ViewGroup.LayoutParams.WRAP_CONTENT);
        tlp.leftMargin = dp(6);
        today.setOnClickListener(new View.OnClickListener() {
            @Override
            public void onClick(View v) {
                if (data == null) return;
                displayWeek = currentWeek;
                renderWeek();
            }
        });
        badges.addView(today, tlp);

        head.addView(UiKit.appBar(this, "课表", badges,
                new String[]{"＋", "≡"},
                new View.OnClickListener[]{
                        new View.OnClickListener() {
                            @Override
                            public void onClick(View v) {
                                openAnalyze("");
                            }
                        },
                        new View.OnClickListener() {
                            @Override
                            public void onClick(View v) {
                                UiKit.menuSheet(MainActivity.this);
                            }
                        }
                },
                new View.OnClickListener() {
                    @Override
                    public void onClick(View v) {
                        startActivity(new Intent(MainActivity.this,
                                TimetableActivity.class));
                    }
                }));
        return head;
    }

    private TextView icon(String glyph, View.OnClickListener l) {
        TextView t = UiKit.text(this, glyph, 21, 0xFF4E4E5A);
        t.setGravity(Gravity.CENTER);
        t.setPadding(dp(10), dp(4), dp(10), dp(4));
        t.setOnClickListener(l);
        return t;
    }

    private void showWeekPicker() {
        if (data == null) return;
        int total = Math.max(1, data.optInt("weeks", 20));
        final String[] items = new String[total];
        for (int i = 0; i < total; i++) {
            int w = i + 1;
            items[i] = w == currentWeek ? "第 " + w + " 周 · 本周" : "第 " + w + " 周";
        }
        new AlertDialog.Builder(this)
                .setTitle("选择周次")
                .setItems(items, new DialogInterface.OnClickListener() {
                    @Override
                    public void onClick(DialogInterface d, int which) {
                        displayWeek = which + 1;
                        renderWeek();
                    }
                })
                .setNegativeButton("取消", null)
                .show();
    }

    private void load() {
        if (!RustCore.available()) {
            gridBox.removeAllViews();
            gridBox.addView(UiKit.hint(this, "Rust 核心未加载"));
            if (backWeek != null) backWeek.setVisibility(View.GONE);
            return;
        }
        // 轻量预判：屏幕旋转 / 切输入法等「非数据变更」的 Activity 重建，
        // 距上次 load < 3s 直接用已有缓存渲染，避免每次都进线程 + JNI。
        long now = System.currentTimeMillis();
        if (lastTtJson != null && lastSchJson != null
                && now - lastLoadAtMs < 3000L
                && data != null) {
            currentWeek = Math.max(1, data.optInt("current_week", 1));
            if (displayWeek <= 0) displayWeek = currentWeek;
            renderWeek();
            return;
        }
        final Context ctx = this;
        new Thread(new Runnable() {
            @Override
            public void run() {
                final String ttJson = RustCore.timetable(ctx);
                final String schJson = RustCore.scheduleList(ctx);
                ui.post(new Runnable() {
                    @Override
                    public void run() {
                        lastLoadAtMs = System.currentTimeMillis();
                        try {
                            data = ttJson == null ? null : new JSONObject(ttJson);
                            schedule = new JSONArray(schJson == null ? "[]" : schJson);
                        } catch (Throwable t) {
                            data = null;
                            schedule = new JSONArray();
                        }
                        if (data == null) {
                            gridBox.removeAllViews();
                            gridBox.addView(UiKit.hint(MainActivity.this,
                                    "读取课表失败，点「导入课表」"));
                            if (backWeek != null) backWeek.setVisibility(View.GONE);
                            return;
                        }
                        currentWeek = Math.max(1, data.optInt("current_week", 1));
                        if (displayWeek <= 0) displayWeek = currentWeek;
                        // 数据与周次都没变时不重建整页，回到前台更流畅。
                        if (ttJson != null && ttJson.equals(lastTtJson)
                                && schJson != null && schJson.equals(lastSchJson)
                                && displayWeek == lastRenderWeek) {
                            return;
                        }
                        lastTtJson = ttJson;
                        lastSchJson = schJson;
                        renderWeek();
                    }
                });
            }
        }, "lancai-home-load").start();
    }

    private void renderWeek() {
        if (gridBox == null || data == null) return;
        int total = Math.max(1, data.optInt("weeks", 20));
        displayWeek = Math.max(1, Math.min(displayWeek, total));
        currentWeek = Math.max(1, Math.min(data.optInt("current_week", 1), total));

        weekLabel.setText(displayWeek == currentWeek
                ? "第 " + displayWeek + " 周"
                : "第 " + displayWeek + " 周 · 本周 " + currentWeek);
        lastRenderWeek = displayWeek;
        if (backWeek != null) {
            backWeek.setVisibility(displayWeek == currentWeek ? View.GONE : View.VISIBLE);
        }

        gridBox.removeAllViews();
        JSONArray courses = data.optJSONArray("courses");
        boolean hasActs = schedule != null && schedule.length() > 0;
        if ((courses == null || courses.length() == 0) && !hasActs) {
            LinearLayout empty = UiKit.vertical(this);
            empty.setGravity(Gravity.CENTER);
            empty.setPadding(0, dp(30), 0, dp(30));
            TextView tip = UiKit.text(this, "还没有课表数据", 14, UiKit.TEXT);
            tip.setTypeface(null, Typeface.BOLD);
            tip.setGravity(Gravity.CENTER);
            empty.addView(tip);
            TextView sub = UiKit.text(this, "导入 .ics / JSON / CSV 或粘贴文本即可生成课表", 12, UiKit.SUB);
            sub.setGravity(Gravity.CENTER);
            sub.setPadding(0, dp(6), 0, dp(14));
            empty.addView(sub);
            Button go = UiKit.button(this, "导入课表", UiKit.brand(this),
                    new View.OnClickListener() {
                        @Override
                        public void onClick(View v) {
                            startActivity(new Intent(MainActivity.this,
                                    TimetableActivity.class));
                        }
                    });
            empty.addView(go, new LinearLayout.LayoutParams(dp(180), dp(44)));
            gridBox.addView(empty);
            return;
        }
        try {
            gridBox.addView(TimetableGrid.build(this, data, schedule, displayWeek,
                    new TimetableGrid.Listener() {
                        @Override
                        public void onCourseClick(JSONObject course) {
                            // 首页只展示；详情去完整课表页。
                            UiKit.toast(MainActivity.this,
                                    course.optString("name", "课程"));
                        }

                        @Override
                        public void onActivityClick(JSONObject todo) {
                            startActivity(new Intent(MainActivity.this,
                                    ScheduleActivity.class));
                        }
                    }));
        } catch (Throwable t) {
            gridBox.removeAllViews();
            gridBox.addView(UiKit.warnLine(this, "课表渲染失败：" + t));
        }
    }

    // ------------------------------------------------------------------
    // 分析弹窗
    // ------------------------------------------------------------------

    private void openAnalyze(String prefill) {
        LinearLayout sheet = UiKit.vertical(this);
        GradientDrawable bg = new GradientDrawable();
        bg.setColor(Color.WHITE);
        bg.setCornerRadii(new float[]{dp(22), dp(22), dp(22), dp(22), 0, 0, 0, 0});
        sheet.setBackground(bg);
        sheet.addView(UiKit.handle(this));

        TextView head = UiKit.serif(this, "分析通知", 18, UiKit.INK);
        head.setTypeface(Typeface.create(Typeface.SERIF, Typeface.BOLD));
        head.setPadding(dp(18), dp(10), dp(18), dp(2));
        sheet.addView(head);

        LinearLayout root = UiKit.vertical(this);
        root.setPadding(dp(16), dp(8), dp(16), dp(18));

        dlgSource = new EditText(this);
        dlgSource.setHint("来源（可选）");
        dlgSource.setSingleLine(true);
        dlgSource.setTextSize(13);
        root.addView(dlgSource);

        dlgText = new EditText(this);
        dlgText.setHint("粘贴通知内容…");
        dlgText.setGravity(Gravity.TOP | Gravity.START);
        dlgText.setMinLines(4);
        dlgText.setMaxLines(8);
        dlgText.setTextSize(14);
        LinearLayout.LayoutParams tp = new LinearLayout.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.WRAP_CONTENT);
        tp.topMargin = dp(6);
        root.addView(dlgText, tp);

        dlgAnalyze = UiKit.button(this, "分析这条消息", UiKit.brand(this),
                new View.OnClickListener() {
                    @Override
                    public void onClick(View v) {
                        analyze();
                    }
                });
        LinearLayout.LayoutParams ap = new LinearLayout.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT, dp(44));
        ap.topMargin = dp(10);
        root.addView(dlgAnalyze, ap);

        resultBox = UiKit.vertical(this);
        root.addView(resultBox);

        recentBox = UiKit.vertical(this);
        root.addView(recentBox);

        ScrollView sc = new ScrollView(this);
        sc.setVerticalScrollBarEnabled(false);
        sc.addView(root, new ViewGroup.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.WRAP_CONTENT));

        int maxH = (int) (getResources().getDisplayMetrics().heightPixels * 0.82f);
        sheet.addView(sc, new LinearLayout.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT, Math.min(maxH, dp(620))));

        analyzeDialog = UiKit.sheet(this, sheet);
        analyzeDialog.show();

        if (prefill != null && !prefill.isEmpty()) {
            dlgText.setText(prefill);
            dlgSource.setText("分享");
            analyze();
        } else {
            loadRecent();
        }
    }

    private void analyze() {
        if (!RustCore.available()) {
            UiKit.toastLong(this, "Rust 核心未加载，无法分析");
            return;
        }
        final String text = dlgText.getText().toString().trim();
        if (TextUtils.isEmpty(text)) {
            UiKit.toast(this, "先粘贴一段文本");
            dlgText.requestFocus();
            return;
        }
        final String source = dlgSource.getText().toString().trim();
        dlgAnalyze.setEnabled(false);
        dlgAnalyze.setText("分析中…");
        final Context ctx = this;
        new Thread(new Runnable() {
            @Override
            public void run() {
                final String json = RustCore.analyze(ctx, source, text, true);
                ui.post(new Runnable() {
                    @Override
                    public void run() {
                        dlgAnalyze.setEnabled(true);
                        dlgAnalyze.setText("分析这条消息");
                        if (json == null) {
                            UiKit.toastLong(MainActivity.this, "分析失败，请查看日志或重装完整 APK");
                            return;
                        }
                        renderOutcome(json, text);
                        loadRecent();
                    }
                });
            }
        }, "lancai-analyze").start();
    }

    private void renderOutcome(String json, final String rawText) {
        try {
            JSONObject o = new JSONObject(json);
            JSONObject s = o.optJSONObject("summary");
            if (s == null) s = new JSONObject();

            resultBox.removeAllViews();
            LinearLayout card = UiKit.card(this);
            LinearLayout head = UiKit.horizontal(this);
            TextView t = UiKit.title(this, "分析结果");
            head.addView(t, new LinearLayout.LayoutParams(0,
                    ViewGroup.LayoutParams.WRAP_CONTENT, 1f));
            boolean important = o.optBoolean("important", false);
            boolean dup = o.optBoolean("duplicate", false);
            head.addView(UiKit.tag(this,
                    dup ? "重复" : (important ? "重要" : "普通"),
                    dup ? UiKit.SUB : (important ? UiKit.brand(this) : 0xFF9AA8A2)));
            card.addView(head);

            String title = s.optString("title", "");
            if (TextUtils.isEmpty(title)) {
                title = rawText.length() > 40 ? rawText.substring(0, 40) + "…" : rawText;
            }
            TextView tt = UiKit.text(this, title, 16, UiKit.TEXT);
            tt.setTypeface(null, android.graphics.Typeface.BOLD);
            tt.setPadding(0, dp(6), 0, dp(2));
            card.addView(tt);

            card.addView(UiKit.hint(this, o.optInt("score", 0) + " 分 · 分类 "
                    + empty(s.optString("category", ""), "其他")
                    + " · 阈值 " + Config.get(this).getThreshold() + " 分"));

            lastCard = buildCard(s, title);
            addField(card, "时间", s.optString("when", ""));
            addField(card, "地点", s.optString("where", ""));
            addField(card, "学分", s.optString("credit", ""));
            addField(card, "名额", s.optString("quota", ""));
            addField(card, "截止", s.optString("deadline", ""));
            addField(card, "报名", s.optString("how", ""));

            final TextView attend = UiKit.text(this, "能否参加：判断中…", 12.5f, UiKit.SUB);
            attend.setPadding(0, dp(4), 0, 0);
            card.addView(attend);
            final Context checkCtx = this;
            new Thread(new Runnable() {
                @Override
                public void run() {
                    final String a = RustCore.enrollCheck(checkCtx, rawText);
                    ui.post(new Runnable() {
                        @Override
                        public void run() {
                            if (TextUtils.isEmpty(a)) {
                                attend.setText("能否参加：暂时无法判断");
                                return;
                            }
                            try {
                                JSONObject j = new JSONObject(a);
                                boolean can = j.optBoolean("can_attend", true);
                                boolean known = j.optBoolean("time_known", false);
                                String reason = j.optString("reason", "");
                                attend.setText("能否参加："
                                        + (known ? (can ? "可以参加" : "可能无法参加")
                                                : "未识别到时间")
                                        + (TextUtils.isEmpty(reason) ? "" : " · " + reason));
                                attend.setTextColor(can ? UiKit.brand(checkCtx) : 0xFFB4573F);
                            } catch (Throwable ignore) {
                                attend.setText("能否参加：暂时无法判断");
                            }
                        }
                    });
                }
            }, "lancai-enroll-home").start();

            String ai = s.optString("aiSummary", "");
            if (!TextUtils.isEmpty(ai)) {
                card.addView(UiKit.divider(this));
                card.addView(UiKit.label(this, "AI 总结"));
                card.addView(UiKit.body(this, ai));
            }

            // 加入日程
            final JSONObject summary = s;
            LinearLayout addRow = UiKit.horizontal(this);
            addRow.setPadding(0, dp(10), 0, 0);
            addScheduleBtn = UiKit.ghostButton(this, "加入日程", UiKit.accent(this),
                    new View.OnClickListener() {
                        @Override
                        public void onClick(View v) {
                            addToSchedule(summary, rawText, false);
                        }
                    });
            addRow.addView(addScheduleBtn, new LinearLayout.LayoutParams(0, dp(38), 1f));
            card.addView(addRow);

            LinearLayout row = UiKit.horizontal(this);
            row.setPadding(0, dp(8), 0, 0);
            Button copy = UiKit.ghostButton(this, "复制要点", UiKit.brand(this),
                    new View.OnClickListener() {
                        @Override
                        public void onClick(View v) {
                            UiKit.copy(MainActivity.this, "澜彩哨兵", lastCard);
                        }
                    });
            Button aiBtn = UiKit.ghostButton(this, "AI 总结", UiKit.brand(this),
                    new View.OnClickListener() {
                        @Override
                        public void onClick(View v) {
                            aiSummary(rawText);
                        }
                    });
            Button push = UiKit.ghostButton(this, "推送", UiKit.brand(this),
                    new View.OnClickListener() {
                        @Override
                        public void onClick(View v) {
                            push(lastCard);
                        }
                    });
            addRowButtons(row, copy, aiBtn, push);
            card.addView(row);

            resultBox.addView(card);

            // 识别到活动/待办（有时间、名额或报名方式）时自动加入日程。
            boolean looksLikeEvent = !s.optString("when", "").isEmpty()
                    || !s.optString("quota", "").isEmpty()
                    || !s.optString("how", "").isEmpty();
            if (looksLikeEvent) {
                addToSchedule(s, rawText, true);
            }
        } catch (Throwable t) {
            UiKit.toastLong(this, "结果解析失败");
        }
    }

    private void addRowButtons(LinearLayout row, Button a, Button b, Button c) {
        row.addView(a, new LinearLayout.LayoutParams(0, dp(38), 1f));
        LinearLayout.LayoutParams lp = new LinearLayout.LayoutParams(0, dp(38), 1f);
        lp.leftMargin = dp(6);
        row.addView(b, lp);
        LinearLayout.LayoutParams lp2 = new LinearLayout.LayoutParams(0, dp(38), 1f);
        lp2.leftMargin = dp(6);
        row.addView(c, lp2);
    }

    private void aiSummary(final String text) {
        if (TextUtils.isEmpty(text)) return;
        Config cfg = Config.get(this);
        if (!cfg.hasAiKey() && !"custom".equals(cfg.getAiProvider())) {
            UiKit.toastLong(this, "先在「菜单 → 推送与 AI」里填写 API Key（或选自定义/本地）");
            return;
        }
        UiKit.toast(this, "正在请求 AI…");
        final Context ctx = this;
        new Thread(new Runnable() {
            @Override
            public void run() {
                final String json = RustCore.ai(ctx, text);
                ui.post(new Runnable() {
                    @Override
                    public void run() {
                        try {
                            JSONObject o = new JSONObject(json == null ? "{}" : json);
                            if (o.optBoolean("ok")) {
                                UiKit.infoDialog(MainActivity.this, "AI 总结", o.optString("text", ""));
                            } else {
                                UiKit.infoDialog(MainActivity.this, "AI 总结失败",
                                        o.optString("error", "未知错误"));
                            }
                        } catch (Throwable e) {
                            UiKit.toastLong(MainActivity.this, "AI 返回异常");
                        }
                    }
                });
            }
        }, "lancai-ai").start();
    }

    private void push(String content) {
        if (!Config.get(this).hasPushToken()) {
            UiKit.toastLong(this, "先在「菜单 → 推送与 AI」里填写 PushPlus Token");
            return;
        }
        final String body = content;
        final Context ctx = this;
        UiKit.toast(this, "正在推送…");
        new Thread(new Runnable() {
            @Override
            public void run() {
                String title = "澜彩哨兵";
                try {
                    String first = body.split("\n", 2)[0];
                    if (!TextUtils.isEmpty(first)) title = first;
                } catch (Throwable ignore) {
                }
                final String json = RustCore.push(ctx, title, body);
                ui.post(new Runnable() {
                    @Override
                    public void run() {
                        try {
                            JSONObject o = new JSONObject(json == null ? "{}" : json);
                            boolean ok = o.optBoolean("ok", false);
                            UiKit.infoDialog(MainActivity.this,
                                    ok ? "推送成功" : "推送失败", o.optString("detail", ""));
                        } catch (Throwable e) {
                            UiKit.toastLong(MainActivity.this, "推送返回异常");
                        }
                    }
                });
            }
        }, "lancai-push").start();
    }

    // ------------------------------------------------------------------
    // 自动/手动加入日程
    // ------------------------------------------------------------------

    private void addToSchedule(final JSONObject s, final String rawText, final boolean auto) {
        final Context ctx = this;
        new Thread(new Runnable() {
            @Override
            public void run() {
                try {
                    JSONObject o = new JSONObject();
                    String title = s.optString("title", "");
                    if (TextUtils.isEmpty(title)) {
                        title = rawText.length() > 30 ? rawText.substring(0, 30) + "…" : rawText;
                    }
                    o.put("title", title);

                    StringBuilder note = new StringBuilder();
                    String where = s.optString("where", "");
                    String quota = s.optString("quota", "");
                    String how = s.optString("how", "");
                    if (!where.isEmpty()) note.append("地点：").append(where).append("\n");
                    if (!quota.isEmpty()) note.append("名额：").append(quota).append("\n");
                    if (!how.isEmpty()) note.append("报名：").append(how).append("\n");
                    note.append(rawText);
                    o.put("note", note.toString());

                    String when = s.optString("when", "");
                    if (!when.isEmpty()) o.put("due", when);

                    boolean activity = !quota.isEmpty() || !how.isEmpty()
                            || !s.optString("credit", "").isEmpty();
                    String url = extractUrl(rawText);
                    if (!url.isEmpty()) {
                        activity = true;
                        o.put("url", url);
                    }
                    o.put("kind", activity ? "activity" : "todo");
                    o.put("remind_min", 10);

                    // 去重：同标题已存在则跳过。
                    String listJson = RustCore.scheduleList(ctx);
                    boolean dup = false;
                    try {
                        JSONArray arr = new JSONArray(listJson == null ? "[]" : listJson);
                        for (int i = 0; i < arr.length(); i++) {
                            if (title.equals(arr.optJSONObject(i).optString("title", ""))) {
                                dup = true;
                                break;
                            }
                        }
                    } catch (Throwable ignore) {
                    }
                    if (dup) {
                        if (!auto) UiKit.toast(ctx, "日程里已有同名条目");
                        markAdded(auto);
                        return;
                    }

                    String resp = RustCore.scheduleSave(ctx, o.toString());
                    boolean ok = false;
                    try {
                        ok = new JSONObject(resp == null ? "{}" : resp).optBoolean("ok", false);
                    } catch (Throwable ignore) {
                    }
                    if (ok) {
                        ReminderScheduler.reschedule(ctx);
                    }
                    final boolean success = ok;
                    ui.post(new Runnable() {
                        @Override
                        public void run() {
                            if (success) {
                                load();
                                UiKit.toast(ctx, auto ? "已自动加入日程" : "已加入日程");
                            } else if (!auto) {
                                UiKit.toastLong(ctx, "加入失败");
                            }
                            if (success) markAdded(auto);
                        }
                    });
                } catch (Throwable t) {
                    if (!auto) UiKit.toastLong(ctx, "加入失败");
                }
            }
        }, "lancai-add-schedule").start();
    }

    private void markAdded(boolean auto) {
        if (addScheduleBtn != null) {
            addScheduleBtn.setText(auto ? "已自动加入日程" : "已加入日程");
            addScheduleBtn.setEnabled(false);
        }
    }

    private String extractUrl(String text) {
        if (text == null) return "";
        Matcher m = URL_RE.matcher(text);
        return m.find() ? m.group() : "";
    }

    // ------------------------------------------------------------------
    // 最近记录
    // ------------------------------------------------------------------

    private void loadRecent() {
        if (!RustCore.available() || recentBox == null) return;
        final Context ctx = this;
        new Thread(new Runnable() {
            @Override
            public void run() {
                final String json = RustCore.recent(ctx, 10);
                ui.post(new Runnable() {
                    @Override
                    public void run() {
                        renderRecent(json);
                    }
                });
            }
        }, "lancai-recent").start();
    }

    private void renderRecent(String json) {
        recentBox.removeAllViews();
        if (json == null) {
            recentBox.addView(UiKit.hint(this, "读取失败"));
            return;
        }
        try {
            JSONArray arr = new JSONArray(json);
            if (arr.length() == 0) return;
            recentBox.addView(UiKit.label(this, "最近分析"));
            for (int i = 0; i < arr.length(); i++) {
                final JSONObject m = arr.optJSONObject(i);
                if (m == null) continue;
                if (i > 0) recentBox.addView(UiKit.divider(this));
                LinearLayout row = UiKit.vertical(this);
                row.setPadding(0, dp(7), 0, dp(7));
                LinearLayout meta = UiKit.horizontal(this);
                meta.addView(UiKit.text(this, UiKit.hhmm(m.optLong("time", 0)),
                        11, UiKit.SUB));
                TextView src = UiKit.text(this,
                        "  " + empty(m.optString("group", ""), "手动输入"), 11, UiKit.SUB);
                src.setSingleLine(true);
                src.setEllipsize(TextUtils.TruncateAt.END);
                meta.addView(src);
                row.addView(meta);
                TextView body = UiKit.text(this,
                        abbreviate(m.optString("text", ""), 90), 12.5f, UiKit.TEXT);
                row.addView(body);
                row.setOnClickListener(new View.OnClickListener() {
                    @Override
                    public void onClick(View v) {
                        if (dlgText != null) {
                            dlgText.setText(m.optString("text", ""));
                            dlgSource.setText(m.optString("group", ""));
                            dlgText.requestFocus();
                        }
                    }
                });
                recentBox.addView(row);
            }
        } catch (Throwable t) {
            recentBox.addView(UiKit.hint(this, "记录格式异常"));
        }
    }

    private void handleIntent(Intent intent) {
        if (intent == null) return;
        if (!Intent.ACTION_SEND.equals(intent.getAction())) return;
        CharSequence cs = intent.getCharSequenceExtra(Intent.EXTRA_TEXT);
        if (cs == null) return;
        String text = cs.toString();
        if (text.trim().isEmpty()) return;
        openAnalyze(text);
    }

    private String buildCard(JSONObject s, String title) {
        StringBuilder b = new StringBuilder();
        b.append(empty(s.optString("category", ""), "其他")).append("：").append(title);
        appendField(b, "时间", s.optString("when", ""));
        appendField(b, "地点", s.optString("where", ""));
        appendField(b, "学分", s.optString("credit", ""));
        appendField(b, "名额", s.optString("quota", ""));
        appendField(b, "截止", s.optString("deadline", ""));
        appendField(b, "报名", s.optString("how", ""));
        return b.toString();
    }

    private void appendField(StringBuilder b, String label, String value) {
        if (TextUtils.isEmpty(value)) return;
        b.append('\n').append(label).append('：').append(value);
    }

    private void addField(LinearLayout card, String label, String value) {
        if (TextUtils.isEmpty(value)) return;
        LinearLayout row = UiKit.horizontal(this);
        row.setPadding(0, dp(3), 0, 0);
        TextView k = UiKit.text(this, label, 12.5f, UiKit.SUB);
        k.setWidth(dp(46));
        row.addView(k);
        TextView v = UiKit.text(this, value, 13, UiKit.TEXT);
        row.addView(v, new LinearLayout.LayoutParams(0,
                ViewGroup.LayoutParams.WRAP_CONTENT, 1f));
        card.addView(row);
    }

    private String empty(String s, String fallback) {
        return TextUtils.isEmpty(s) ? fallback : s;
    }

    private String abbreviate(String s, int n) {
        if (s == null) return "";
        String t = s.replace('\n', ' ').trim();
        return t.length() > n ? t.substring(0, n) + "…" : t;
    }

    private int dp(float v) {
        return UiKit.dp(this, v);
    }
}
