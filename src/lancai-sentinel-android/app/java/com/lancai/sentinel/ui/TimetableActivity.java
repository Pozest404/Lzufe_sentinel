package com.lancai.sentinel.ui;

// [OWNER: UI] 完整课表页：星课程表风格周视图，同一格里同时显示课程与日程/活动。
// 解析、存储、导入都在 Rust；这里只负责渲染、文件读取和对话框。

import android.app.Activity;
import android.app.AlertDialog;
import android.content.ClipData;
import android.content.ClipboardManager;
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

import com.lancai.sentinel.core.RustCore;

import org.json.JSONArray;
import org.json.JSONObject;

import java.io.ByteArrayOutputStream;
import java.io.InputStream;
import java.text.SimpleDateFormat;
import java.util.ArrayList;
import java.util.Calendar;
import java.util.Date;
import java.util.List;
import java.util.Locale;

public class TimetableActivity extends Activity {

    private static final int REQ_FILE = 9201;
    private static final String[] DAY_NAMES = {"一", "二", "三", "四", "五", "六", "日"};

    private final Handler ui = new Handler(Looper.getMainLooper());
    private LinearLayout weekStrip, gridBox;
    private TextView weekLabel, termHint, backWeek;
    private WeekSwipe weekSwipe;
    private JSONObject data;
    private JSONArray schedule;
    private int displayWeek;
    private int currentWeek = 1;
    private String lastTt, lastSch;
    private int lastWeek = -1;

    private EditText importEdit;
    private AlertDialog importDlg;
    private String pendingAction;

    @Override
    protected void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);
        render();
        load();
        String act = getIntent() == null ? null : getIntent().getStringExtra("action");
        pendingAction = act;
    }

    /** 从底部功能面板进入时，加载完成后直接弹出对应对话框。 */
    private void runPending() {
        String act = pendingAction;
        pendingAction = null;
        if (act == null) return;
        if ("import".equals(act)) importDialog();
        else if ("term".equals(act)) termDialog();
    }

    private void render() {
        RelativeLayout shell = new RelativeLayout(this);
        shell.setBackgroundColor(Color.WHITE);

        LinearLayout head = UiKit.vertical(this);
        head.setId(0x2001);
        head.setBackgroundColor(Color.WHITE);
        head.addView(lightBar(), new LinearLayout.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.WRAP_CONTENT));
        head.addView(headerCard(), new LinearLayout.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.WRAP_CONTENT));
        RelativeLayout.LayoutParams hlp = new RelativeLayout.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.WRAP_CONTENT);
        hlp.addRule(RelativeLayout.ALIGN_PARENT_TOP);
        shell.addView(head, hlp);

        View hline = new View(this);
        hline.setBackgroundColor(UiKit.HAIRLINE);
        hline.setId(0x2003);
        RelativeLayout.LayoutParams lineLp = new RelativeLayout.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT, Math.max(1, dp(1)));
        lineLp.addRule(RelativeLayout.BELOW, 0x2001);
        shell.addView(hline, lineLp);

        ScrollView scroll = new ScrollView(this);
        scroll.setFillViewport(true);
        scroll.setClipToPadding(false);
        scroll.setVerticalScrollBarEnabled(false);

        LinearLayout root = UiKit.vertical(this);

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

        scroll.addView(root, new ViewGroup.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.WRAP_CONTENT));
        RelativeLayout.LayoutParams slp = new RelativeLayout.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.MATCH_PARENT);
        slp.addRule(RelativeLayout.BELOW, 0x2003);
        slp.addRule(RelativeLayout.ABOVE, 0x2002);
        shell.addView(scroll, slp);

        LinearLayout nav = UiKit.bottomNav(this, "timetable");
        nav.setId(0x2002);
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

    private View lightBar() {
        LinearLayout bar = UiKit.horizontal(this);
        bar.setPadding(dp(8), dp(8), dp(6), 0);

        TextView back = UiKit.text(this, "‹", 24, 0xFF33333A);
        back.setGravity(Gravity.CENTER);
        back.setPadding(dp(8), 0, dp(8), dp(4));
        back.setOnClickListener(new View.OnClickListener() {
            @Override
            public void onClick(View v) {
                finish();
            }
        });
        bar.addView(back);

        TextView title = UiKit.text(this, "课表", 20, 0xFF1B1B20);
        title.setTypeface(Typeface.create(Typeface.SERIF, Typeface.BOLD));
        bar.addView(title);
        bar.addView(new View(this), new LinearLayout.LayoutParams(0,
                ViewGroup.LayoutParams.WRAP_CONTENT, 1f));

        TextView term = UiKit.text(this, "设置", 12.5f, 0xFF5B5BD6);
        term.setPadding(dp(10), dp(6), dp(10), dp(6));
        term.setOnClickListener(new View.OnClickListener() {
            @Override
            public void onClick(View v) {
                termDialog();
            }
        });
        bar.addView(term);

        TextView imp = UiKit.text(this, "↓", 20, 0xFF4E4E5A);
        imp.setGravity(Gravity.CENTER);
        imp.setPadding(dp(10), dp(2), dp(10), dp(2));
        imp.setOnClickListener(new View.OnClickListener() {
            @Override
            public void onClick(View v) {
                importDialog();
            }
        });
        bar.addView(imp);
        return bar;
    }

    private View headerCard() {
        LinearLayout head = UiKit.vertical(this);
        head.setPadding(dp(14), dp(6), dp(8), dp(2));

        LinearLayout meta = UiKit.horizontal(this);
        weekLabel = UiKit.text(this, "第 1 周", 12, 0xFF5B5BD6);
        weekLabel.setTypeface(null, Typeface.BOLD);
        weekLabel.setGravity(Gravity.CENTER);
        weekLabel.setSingleLine(true);
        weekLabel.setPadding(dp(10), dp(4), dp(10), dp(4));
        UiKit.rounded(weekLabel, this, 0xFFEAEAFB, 0, 10);
        meta.addView(weekLabel);

        TextView today = UiKit.text(this, "今天", 11.5f, 0xFF6E6E7A);
        today.setGravity(Gravity.CENTER);
        today.setPadding(dp(10), dp(4), dp(10), dp(4));
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
        meta.addView(today, tlp);

        termHint = UiKit.text(this, "学期未设置", 10.5f, 0xFF9A9AA6);
        termHint.setPadding(dp(8), 0, 0, 0);
        termHint.setSingleLine(true);
        termHint.setEllipsize(TextUtils.TruncateAt.END);
        meta.addView(termHint, new LinearLayout.LayoutParams(0,
                ViewGroup.LayoutParams.WRAP_CONTENT, 1f));
        head.addView(meta);

        HorizontalScrollView hs = new HorizontalScrollView(this);
        hs.setHorizontalScrollBarEnabled(false);
        weekStrip = UiKit.horizontal(this);
        weekStrip.setPadding(0, dp(7), 0, dp(1));
        hs.addView(weekStrip);
        head.addView(hs);
        return head;
    }

    private void buildWeekStrip(int total) {
        if (weekStrip == null) return;
        weekStrip.removeAllViews();
        for (int w = 1; w <= Math.max(1, total); w++) {
            final int week = w;
            boolean sel = w == displayWeek;
            TextView chip = UiKit.text(this, String.valueOf(w), 11.5f,
                    sel ? 0xFF5B5BD6 : 0xFF8E8E9A);
            chip.setGravity(Gravity.CENTER);
            chip.setTypeface(null, sel ? Typeface.BOLD : Typeface.NORMAL);
            UiKit.rounded(chip, this, sel ? 0xFFEAEAFB : 0xFFFFFFFF,
                    sel ? 0 : 0xFFEDEDF2, 10);
            LinearLayout.LayoutParams lp = new LinearLayout.LayoutParams(dp(30), dp(24));
            lp.rightMargin = dp(6);
            chip.setOnClickListener(new View.OnClickListener() {
                @Override
                public void onClick(View v) {
                    displayWeek = week;
                    renderWeek();
                }
            });
            weekStrip.addView(chip, lp);
        }
    }

    private void load() {
        if (!RustCore.available()) {
            showEmpty("Rust 核心未加载，课表不可用");
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
                        try {
                            data = ttJson == null ? null : new JSONObject(ttJson);
                            schedule = new JSONArray(schJson == null ? "[]" : schJson);
                        } catch (Throwable t) {
                            data = null;
                            schedule = new JSONArray();
                        }
                        if (data == null) {
                            showEmpty("读取课表失败");
                            return;
                        }
                        currentWeek = Math.max(1, data.optInt("current_week", 1));
                        if (displayWeek <= 0) displayWeek = currentWeek;
                        // 数据与周次都没变时跳过重建，返回本页更顺。
                        if (ttJson != null && ttJson.equals(lastTt)
                                && schJson != null && schJson.equals(lastSch)
                                && displayWeek == lastWeek) {
                            return;
                        }
                        lastTt = ttJson;
                        lastSch = schJson;
                        renderWeek();
                        runPending();
                    }
                });
            }
        }, "lancai-timetable-load").start();
    }

    private void showEmpty(String text) {
        if (gridBox == null) return;
        gridBox.removeAllViews();
        if (weekLabel != null) weekLabel.setText("第 1 周");
        if (termHint != null) termHint.setText("学期未设置");
        if (weekStrip != null) weekStrip.removeAllViews();
        if (backWeek != null) backWeek.setVisibility(View.GONE);
        gridBox.addView(UiKit.hint(this, text));
    }

    private void renderWeek() {
        if (gridBox == null || data == null) return;
        int total = Math.max(1, data.optInt("weeks", 20));
        displayWeek = Math.max(1, Math.min(displayWeek, total));
        currentWeek = Math.max(1, Math.min(data.optInt("current_week", 1), total));

        weekLabel.setText(displayWeek == currentWeek
                ? "第 " + displayWeek + " 周"
                : "第 " + displayWeek + " 周 · 当前第 " + currentWeek + " 周");
        lastWeek = displayWeek;
        termHint.setText(termText(total));
        buildWeekStrip(total);
        if (backWeek != null) {
            backWeek.setVisibility(displayWeek == currentWeek ? View.GONE : View.VISIBLE);
        }

        gridBox.removeAllViews();
        JSONArray courses = data.optJSONArray("courses");
        boolean hasActs = schedule != null && schedule.length() > 0;
        if ((courses == null || courses.length() == 0) && !hasActs) {
            TextView tip = UiKit.text(this, "还没有课程，点右上角「导入」添加", 12.5f, UiKit.SUB);
            tip.setGravity(Gravity.CENTER);
            tip.setPadding(0, dp(28), 0, dp(28));
            gridBox.addView(tip);
            return;
        }
        gridBox.addView(TimetableGrid.build(this, data, schedule, displayWeek,
                new TimetableGrid.Listener() {
                    @Override
                    public void onCourseClick(JSONObject course) {
                        courseDialog(course, course.optInt("idx", -1));
                    }

                    @Override
                    public void onActivityClick(JSONObject todo) {
                        startActivity(new Intent(TimetableActivity.this,
                                ScheduleActivity.class));
                    }
                }));
    }

    private String termText(int total) {
        String start = data.optString("term_start", "");
        if (TextUtils.isEmpty(start)) return "学期未设置 · 点「设置」";
        try {
            Date d = new SimpleDateFormat("yyyy-MM-dd", Locale.CHINA).parse(start);
            Calendar c = Calendar.getInstance();
            if (d != null) c.setTime(d);
            return (c.get(Calendar.MONTH) + 1) + "月" + c.get(Calendar.DAY_OF_MONTH)
                    + "日开学 · 共 " + total + " 周";
        } catch (Throwable t) {
            return "共 " + total + " 周";
        }
    }

    // ------------------------------------------------------------------
    // 课程详情
    // ------------------------------------------------------------------

    private void courseDialog(final JSONObject c, final int idx) {
        String name = c.optString("name", "课程");
        StringBuilder sb = new StringBuilder();
        String teacher = c.optString("teacher", "");
        String loc = c.optString("location", "");
        if (!teacher.isEmpty()) sb.append("教师：").append(teacher).append("\n");
        if (!loc.isEmpty()) sb.append("地点：").append(loc).append("\n");
        sb.append("时间：周").append(DAY_NAMES[Math.max(0, Math.min(6,
                c.optInt("day", 1) - 1))]);
        int s = c.optInt("start_section", 1);
        int e = Math.max(s, c.optInt("end_section", s));
        if (s == e) {
            sb.append(" 第").append(s).append("节");
        } else {
            sb.append(" 第").append(s).append("-").append(e).append("节");
        }
        String st = c.optString("start_time", "");
        String et = c.optString("end_time", "");
        if (!st.isEmpty() || !et.isEmpty()) {
            sb.append(" ").append(st).append("-").append(et);
        }
        sb.append("\n周次：").append(weeksText(c.optJSONArray("weeks")));

        AlertDialog.Builder b = new AlertDialog.Builder(this)
                .setTitle(name)
                .setMessage(sb.toString())
                .setPositiveButton("关闭", null);
        if (idx >= 0) {
            b.setNeutralButton("删除", new DialogInterface.OnClickListener() {
                @Override
                public void onClick(DialogInterface d, int which) {
                    confirmDelete(idx, name);
                }
            });
        }
        b.create().show();
    }

    private String weeksText(JSONArray ws) {
        if (ws == null || ws.length() == 0) return "每周";
        List<Integer> list = new ArrayList<>();
        for (int i = 0; i < ws.length(); i++) {
            list.add(ws.optInt(i, 0));
        }
        int first = list.get(0);
        int last = list.get(list.size() - 1);
        boolean odd = true;
        boolean even = true;
        for (int w : list) {
            if (w % 2 == 0) odd = false;
            if (w % 2 == 1) even = false;
        }
        String base = first == last ? (first + "周")
                : (first + "-" + last + "周");
        if (odd && !even) return base + "（单周）";
        if (even && !odd) return base + "（双周）";
        return base;
    }

    private void confirmDelete(final int idx, String name) {
        UiKit.confirm(this, "删除课程", "确定删除「" + name + "」？", "删除", new Runnable() {
            @Override
            public void run() {
                final Context ctx = TimetableActivity.this;
                new Thread(new Runnable() {
                    @Override
                    public void run() {
                        final String json = RustCore.timetableDelete(ctx, idx);
                        ui.post(new Runnable() {
                            @Override
                            public void run() {
                                boolean ok = false;
                                try {
                                    ok = new JSONObject(json == null ? "{}" : json)
                                            .optBoolean("ok", false);
                                } catch (Throwable ignore) {
                                }
                                if (!ok) UiKit.toast(TimetableActivity.this, "删除失败");
                                load();
                            }
                        });
                    }
                }, "lancai-timetable-delete").start();
            }
        });
    }

    // ------------------------------------------------------------------
    // 学期设置
    // ------------------------------------------------------------------

    private void termDialog() {
        if (data == null) return;
        LinearLayout form = UiKit.vertical(this);
        form.setPadding(dp(18), 0, dp(18), 0);
        final EditText etStart = new EditText(this);
        etStart.setHint("第一周周一，yyyy-MM-dd");
        etStart.setText(data.optString("term_start", ""));
        etStart.setTextSize(14);
        final EditText etWeeks = new EditText(this);
        etWeeks.setHint("总周数，例如 18");
        etWeeks.setText(String.valueOf(data.optInt("weeks", 20)));
        etWeeks.setTextSize(14);
        etWeeks.setInputType(android.text.InputType.TYPE_CLASS_NUMBER);

        form.addView(UiKit.label(this, "学期开始日期"));
        form.addView(etStart);
        form.addView(UiKit.label(this, "总周数"));
        form.addView(etWeeks);
        form.addView(UiKit.hint(this, "日期会自动对齐到该周的周一；导入 ICS 时通常已自动设置。"));

        AlertDialog.Builder b = new AlertDialog.Builder(this)
                .setTitle("学期设置")
                .setView(form)
                .setPositiveButton("保存", null)
                .setNegativeButton("取消", null)
                .setNeutralButton("清空课表", new DialogInterface.OnClickListener() {
                    @Override
                    public void onClick(DialogInterface d, int which) {
                        confirmClear();
                    }
                });
        final AlertDialog dialog = b.create();
        dialog.setOnShowListener(new DialogInterface.OnShowListener() {
            @Override
            public void onShow(DialogInterface d) {
                Button save = dialog.getButton(DialogInterface.BUTTON_POSITIVE);
                save.setOnClickListener(new View.OnClickListener() {
                    @Override
                    public void onClick(View v) {
                        String start = etStart.getText().toString().trim();
                        int weeks;
                        try {
                            weeks = Integer.parseInt(etWeeks.getText().toString().trim());
                        } catch (Throwable t) {
                            UiKit.toast(TimetableActivity.this, "总周数填数字");
                            return;
                        }
                        saveTerm(start, weeks, dialog);
                    }
                });
            }
        });
        dialog.show();
    }

    private void saveTerm(final String start, final int weeks, final AlertDialog dialog) {
        final Context ctx = this;
        new Thread(new Runnable() {
            @Override
            public void run() {
                final String json = RustCore.timetableSetTerm(ctx, start, weeks);
                ui.post(new Runnable() {
                    @Override
                    public void run() {
                        boolean ok = false;
                        String err = "";
                        try {
                            JSONObject o = new JSONObject(json == null ? "{}" : json);
                            ok = o.optBoolean("ok", false);
                            err = o.optString("error", "");
                        } catch (Throwable ignore) {
                        }
                        if (!ok) {
                            UiKit.toastLong(TimetableActivity.this,
                                    err.isEmpty() ? "保存失败" : err);
                            return;
                        }
                        dialog.dismiss();
                        displayWeek = 0;
                        load();
                    }
                });
            }
        }, "lancai-timetable-term").start();
    }

    private void confirmClear() {
        UiKit.confirm(this, "清空课表", "会删除全部课程，不可恢复。", "清空", new Runnable() {
            @Override
            public void run() {
                final Context ctx = TimetableActivity.this;
                new Thread(new Runnable() {
                    @Override
                    public void run() {
                        RustCore.timetableClear(ctx);
                        ui.post(new Runnable() {
                            @Override
                            public void run() {
                                displayWeek = 0;
                                load();
                            }
                        });
                    }
                }, "lancai-timetable-clear").start();
            }
        });
    }

    // ------------------------------------------------------------------
    // 导入
    // ------------------------------------------------------------------

    private void importDialog() {
        LinearLayout form = UiKit.vertical(this);
        form.setPadding(dp(18), 0, dp(18), 0);
        form.addView(UiKit.hint(this,
                "支持星课程表导出的 .ics、课程表 JSON/CSV，或直接粘贴课表文本。导入会覆盖当前课表。"));

        importEdit = new EditText(this);
        importEdit.setHint("粘贴 .ics 内容或课表文本…");
        importEdit.setGravity(Gravity.TOP | Gravity.START);
        importEdit.setMinLines(7);
        importEdit.setMaxLines(12);
        importEdit.setTextSize(12.5f);
        importEdit.setTextColor(UiKit.TEXT);
        importEdit.setHintTextColor(0xFF9AA8A2);
        form.addView(importEdit, new LinearLayout.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.WRAP_CONTENT));

        LinearLayout tools = UiKit.horizontal(this);
        tools.setPadding(0, dp(8), 0, 0);
        Button file = UiKit.ghostButton(this, "选择文件", UiKit.brand(this),
                new View.OnClickListener() {
                    @Override
                    public void onClick(View v) {
                        pickFile();
                    }
                });
        Button paste = UiKit.ghostButton(this, "粘贴剪贴板", UiKit.SUB,
                new View.OnClickListener() {
                    @Override
                    public void onClick(View v) {
                        pasteClipboard();
                    }
                });
        LinearLayout.LayoutParams bp = new LinearLayout.LayoutParams(0, dp(38), 1f);
        bp.rightMargin = dp(6);
        tools.addView(file, bp);
        tools.addView(paste, new LinearLayout.LayoutParams(0, dp(38), 1f));
        form.addView(tools);

        AlertDialog.Builder b = new AlertDialog.Builder(this)
                .setTitle("导入课表")
                .setView(form)
                .setPositiveButton("导入", null)
                .setNegativeButton("取消", null);
        importDlg = b.create();
        importDlg.setOnDismissListener(new DialogInterface.OnDismissListener() {
            @Override
            public void onDismiss(DialogInterface d) {
                importEdit = null;
                importDlg = null;
            }
        });
        importDlg.setOnShowListener(new DialogInterface.OnShowListener() {
            @Override
            public void onShow(DialogInterface d) {
                Button ok = importDlg.getButton(DialogInterface.BUTTON_POSITIVE);
                ok.setOnClickListener(new View.OnClickListener() {
                    @Override
                    public void onClick(View v) {
                        doImport();
                    }
                });
            }
        });
        importDlg.show();
    }

    private void doImport() {
        if (importEdit == null) return;
        String text = importEdit.getText().toString();
        if (text.trim().isEmpty()) {
            UiKit.toast(this, "先选择文件或粘贴课表内容");
            return;
        }
        final String body = text;
        final Button ok = importDlg == null ? null
                : importDlg.getButton(DialogInterface.BUTTON_POSITIVE);
        if (ok != null) {
            ok.setEnabled(false);
            ok.setText("导入中…");
        }
        final Context ctx = this;
        new Thread(new Runnable() {
            @Override
            public void run() {
                final String json = RustCore.timetableImport(ctx, body);
                ui.post(new Runnable() {
                    @Override
                    public void run() {
                        if (ok != null) {
                            ok.setEnabled(true);
                            ok.setText("导入");
                        }
                        boolean success = false;
                        String msg = "导入失败";
                        try {
                            JSONObject o = new JSONObject(json == null ? "{}" : json);
                            success = o.optBoolean("ok", false);
                            if (success) {
                                msg = "已导入 " + o.optInt("count", 0) + " 门课程（"
                                        + o.optString("format", "课表") + "）";
                            } else {
                                msg = o.optString("error", "导入失败");
                            }
                        } catch (Throwable ignore) {
                        }
                        UiKit.toastLong(TimetableActivity.this, msg);
                        if (success) {
                            if (importDlg != null) importDlg.dismiss();
                            displayWeek = 0;
                            load();
                        }
                    }
                });
            }
        }, "lancai-timetable-import").start();
    }

    private void pickFile() {
        try {
            Intent i = new Intent(Intent.ACTION_OPEN_DOCUMENT);
            i.addCategory(Intent.CATEGORY_OPENABLE);
            i.setType("*/*");
            startActivityForResult(i, REQ_FILE);
        } catch (Throwable t) {
            UiKit.toastLong(this, "无法打开文件选择器");
        }
    }

    private void pasteClipboard() {
        try {
            ClipboardManager cm = (ClipboardManager) getSystemService(Context.CLIPBOARD_SERVICE);
            if (cm == null || cm.getPrimaryClip() == null) {
                UiKit.toast(this, "剪贴板为空");
                return;
            }
            ClipData clip = cm.getPrimaryClip();
            if (clip.getItemCount() == 0) {
                UiKit.toast(this, "剪贴板为空");
                return;
            }
            CharSequence s = clip.getItemAt(0).coerceToText(this);
            if (s == null || s.length() == 0) {
                UiKit.toast(this, "剪贴板为空");
                return;
            }
            if (importEdit != null) importEdit.setText(s);
        } catch (Throwable t) {
            UiKit.toastLong(this, "读取剪贴板失败");
        }
    }

    @Override
    protected void onActivityResult(int requestCode, int resultCode, Intent data) {
        super.onActivityResult(requestCode, resultCode, data);
        if (requestCode != REQ_FILE || resultCode != RESULT_OK || data == null) return;
        Uri uri = data.getData();
        if (uri == null) return;
        final Uri fUri = uri;
        new Thread(new Runnable() {
            @Override
            public void run() {
                final String text = readText(fUri);
                ui.post(new Runnable() {
                    @Override
                    public void run() {
                        if (text == null) {
                            UiKit.toastLong(TimetableActivity.this, "读取文件失败");
                        } else if (importEdit != null) {
                            importEdit.setText(text);
                            UiKit.toast(TimetableActivity.this, "文件已载入，点「导入」");
                        }
                    }
                });
            }
        }, "lancai-timetable-file").start();
    }

    private String readText(Uri uri) {
        InputStream in = null;
        try {
            in = getContentResolver().openInputStream(uri);
            if (in == null) return null;
            ByteArrayOutputStream buf = new ByteArrayOutputStream();
            byte[] tmp = new byte[8192];
            int n;
            int total = 0;
            while ((n = in.read(tmp)) > 0) {
                total += n;
                if (total > 3_000_000) break;
                buf.write(tmp, 0, n);
            }
            return decodeBytes(buf.toByteArray());
        } catch (Throwable t) {
            return null;
        } finally {
            if (in != null) {
                try {
                    in.close();
                } catch (Throwable ignore) {
                }
            }
        }
    }

    /**
     * 课表文件编码并不统一：手机端导出多为 UTF-8，Windows/Excel 另存的多是 GBK。
     * 之前一律按 UTF-8 解码，GBK 文件里的中文会整片变成乱码，导入自然“不对”。
     * 顺序：BOM → 严格 UTF-8（失败即非法）→ GB18030。
     */
    private String decodeBytes(byte[] data) {
        if (data.length >= 3 && (data[0] & 0xFF) == 0xEF
                && (data[1] & 0xFF) == 0xBB && (data[2] & 0xFF) == 0xBF) {
            return new String(data, 3, data.length - 3, java.nio.charset.StandardCharsets.UTF_8);
        }
        if (data.length >= 2 && (data[0] & 0xFF) == 0xFF && (data[1] & 0xFF) == 0xFE) {
            return new String(data, 2, data.length - 2, java.nio.charset.StandardCharsets.UTF_16LE);
        }
        if (data.length >= 2 && (data[0] & 0xFF) == 0xFE && (data[1] & 0xFF) == 0xFF) {
            return new String(data, 2, data.length - 2, java.nio.charset.StandardCharsets.UTF_16BE);
        }
        try {
            java.nio.charset.CharsetDecoder dec = java.nio.charset.StandardCharsets.UTF_8
                    .newDecoder()
                    .onMalformedInput(java.nio.charset.CodingErrorAction.REPORT)
                    .onUnmappableCharacter(java.nio.charset.CodingErrorAction.REPORT);
            return dec.decode(java.nio.ByteBuffer.wrap(data)).toString();
        } catch (Throwable notUtf8) {
            try {
                return new String(data, "GB18030");
            } catch (Throwable ignore) {
                return new String(data);
            }
        }
    }

    @Override
    protected void onResume() {
        super.onResume();
        load();
    }

    private int dp(float v) {
        return UiKit.dp(this, v);
    }
}
