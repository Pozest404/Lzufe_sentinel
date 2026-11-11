package com.lancai.sentinel.ui;

// [OWNER: UI] 日程与待办：本地列表 + 新建/编辑 + 完成勾选 + 提醒排期。
// 业务数据全部由 Rust 读写；这里只负责界面与 Intent。

import android.app.Activity;
import android.app.AlertDialog;
import android.app.DatePickerDialog;
import android.app.Dialog;
import android.app.TimePickerDialog;
import android.content.Context;
import android.content.DialogInterface;
import android.content.Intent;
import android.graphics.Paint;
import android.os.Bundle;
import android.os.Handler;
import android.os.Looper;
import android.text.InputType;
import android.text.TextUtils;
import android.view.Gravity;
import android.view.View;
import android.view.ViewGroup;
import android.widget.ArrayAdapter;
import android.widget.Button;
import android.widget.DatePicker;
import android.widget.EditText;
import android.widget.LinearLayout;
import android.widget.ScrollView;
import android.widget.Spinner;
import android.widget.TextView;
import android.widget.TimePicker;

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

public class ScheduleActivity extends Activity {

    private static final String[] KIND_NAMES = {"待办", "第二课堂活动"};
    private static final String[] KIND_IDS = {"todo", "activity"};
    private static final String[] REMIND_NAMES = {
            "不提醒", "准时", "提前 5 分钟", "提前 10 分钟",
            "提前 30 分钟", "提前 1 小时", "提前 1 天"
    };
    private static final int[] REMIND_VALUES = {-1, 0, 5, 10, 30, 60, 1440};

    private LinearLayout listBox;
    private TextView permHint;
    private final Handler ui = new Handler(Looper.getMainLooper());

    @Override
    protected void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);
        NotificationHelper.ensureChannel(this);
        render();
    }

    @Override
    protected void onResume() {
        super.onResume();
        if (listBox != null) load();
    }

    private void render() {
        LinearLayout page = UiKit.vertical(this);
        page.setBackgroundColor(UiKit.BG);
        page.addView(UiKit.topBar(this, "日程与待办", "新建", new View.OnClickListener() {
            @Override
            public void onClick(View v) {
                edit(null);
            }
        }), new LinearLayout.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.WRAP_CONTENT));

        ScrollView scroll = new ScrollView(this);
        scroll.setFillViewport(true);
        LinearLayout root = UiKit.vertical(this);
        root.setPadding(dp(14), dp(8), dp(14), dp(26));
        scroll.addView(root, new ViewGroup.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.WRAP_CONTENT));
        page.addView(scroll, new LinearLayout.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT, 0, 1f));
        setContentView(page);

        root.addView(UiKit.hint(this, "待办与第二课堂活动只保存在本机；到点前会发系统通知。"));

        permHint = UiKit.warnLine(this, "通知权限未开启，到点不会提醒。点这里去开启。");
        permHint.setVisibility(View.GONE);
        permHint.setPadding(0, dp(8), 0, dp(2));
        permHint.setOnClickListener(new View.OnClickListener() {
            @Override
            public void onClick(View v) {
                NotificationHelper.requestPermission(ScheduleActivity.this);
            }
        });
        root.addView(permHint);

        LinearLayout card = UiKit.card(this);
        LinearLayout head = UiKit.horizontal(this);
        head.addView(UiKit.title(this, "我的日程"),
                new LinearLayout.LayoutParams(0, ViewGroup.LayoutParams.WRAP_CONTENT, 1f));
        TextView clear = UiKit.text(this, "清除已完成", 11.5f, UiKit.brand(this));
        clear.setPadding(dp(8), dp(4), 0, dp(4));
        clear.setOnClickListener(new View.OnClickListener() {
            @Override
            public void onClick(View v) {
                confirmClearDone();
            }
        });
        head.addView(clear);
        card.addView(head);

        listBox = UiKit.vertical(this);
        card.addView(listBox);
        LinearLayout.LayoutParams lp = new LinearLayout.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.WRAP_CONTENT);
        lp.topMargin = dp(10);
        root.addView(card, lp);
    }

    private void load() {
        if (!RustCore.available()) {
            renderEmpty("Rust 核心未加载，日程不可用");
            return;
        }
        if (permHint != null && android.os.Build.VERSION.SDK_INT >= 33
                && !NotificationHelper.canNotify(this)) {
            permHint.setVisibility(View.VISIBLE);
        } else if (permHint != null) {
            permHint.setVisibility(View.GONE);
        }
        final Context ctx = this;
        new Thread(new Runnable() {
            @Override
            public void run() {
                final String json = RustCore.scheduleList(ctx);
                // 读列表不触发排期：只在 save/toggle/delete/clearDone 后排。
                ui.post(new Runnable() {
                    @Override
                    public void run() {
                        renderList(json);
                    }
                });
            }
        }, "lancai-schedule-list").start();
    }

    private void renderList(String json) {
        if (listBox == null) return;
        listBox.removeAllViews();
        if (json == null) {
            listBox.addView(UiKit.hint(this, "读取失败，请查看日志"));
            return;
        }
        try {
            JSONArray arr = new JSONArray(json);
            if (arr.length() == 0) {
                renderEmpty("还没有日程。点右上角「新建」添加待办或第二课堂活动。");
                return;
            }
            for (int i = 0; i < arr.length(); i++) {
                final JSONObject o = arr.optJSONObject(i);
                if (o == null) continue;
                if (i > 0) listBox.addView(UiKit.divider(this));
                listBox.addView(itemRow(o));
            }
        } catch (Throwable t) {
            listBox.addView(UiKit.hint(this, "日程格式异常"));
        }
    }

    private void renderEmpty(String text) {
        listBox.removeAllViews();
        listBox.addView(UiKit.hint(this, text));
    }

    private View itemRow(final JSONObject o) {
        final boolean done = o.optBoolean("done", false);
        final boolean activity = "activity".equals(o.optString("kind", "todo"));
        final long id = o.optLong("id", 0);
        final String url = o.optString("url", "");

        LinearLayout row = UiKit.horizontal(this);
        row.setPadding(0, dp(8), 0, dp(8));
        row.setGravity(Gravity.TOP);

        final TextView check = UiKit.text(this, done ? "☑" : "☐", 22,
                done ? UiKit.brand(this) : UiKit.SUB);
        check.setPadding(0, 0, dp(8), 0);
        check.setOnClickListener(new View.OnClickListener() {
            @Override
            public void onClick(View v) {
                toggle(id, !done);
            }
        });
        row.addView(check);

        LinearLayout col = UiKit.vertical(this);
        LinearLayout titleRow = UiKit.horizontal(this);
        TextView title = UiKit.text(this, o.optString("title", ""), 14,
                done ? UiKit.SUB : UiKit.TEXT);
        title.setTypeface(null, android.graphics.Typeface.BOLD);
        if (done) title.setPaintFlags(title.getPaintFlags() | Paint.STRIKE_THRU_TEXT_FLAG);
        titleRow.addView(title, new LinearLayout.LayoutParams(0,
                ViewGroup.LayoutParams.WRAP_CONTENT, 1f));
        titleRow.addView(UiKit.tag(this, activity ? "活动" : "待办",
                activity ? UiKit.accent(this) : UiKit.brand(this)));
        col.addView(titleRow);

        StringBuilder meta = new StringBuilder(activity ? "第二课堂" : "待办");
        long due = o.optLong("due_ms", 0);
        if (due > 0) {
            meta.append(" · ").append(UiKit.ymdhm(due));
            meta.append(" · ").append(remindText(o.optInt("remind_min", 10)));
        } else {
            meta.append(" · 无时间");
        }
        if (activity && o.optBoolean("auto_signup", false)) {
            meta.append(" · 自动报名已开启");
        }
        if (done) meta.append(" · 已完成");
        col.addView(UiKit.text(this, meta.toString(), 11.5f, UiKit.SUB));

        final String note = o.optString("note", "");
        if (!TextUtils.isEmpty(note)) {
            TextView nt = UiKit.text(this, note, 12, UiKit.SUB);
            nt.setPadding(0, dp(2), 0, 0);
            col.addView(nt);
        }
        if (!TextUtils.isEmpty(url)) {
            LinearLayout links = UiKit.horizontal(this);
            links.setPadding(0, dp(5), 0, 0);
            TextView open = UiKit.text(this, "打开活动 ›", 12, UiKit.brand(this));
            open.setOnClickListener(new View.OnClickListener() {
                @Override
                public void onClick(View v) {
                    Intent it = new Intent(ScheduleActivity.this, DektActivity.class);
                    it.putExtra(DektActivity.EXTRA_URL, url);
                    startActivity(it);
                }
            });
            links.addView(open);
            TextView signup = UiKit.text(this, "    一键报名 ›", 12, UiKit.accent(this));
            signup.setOnClickListener(new View.OnClickListener() {
                @Override
                public void onClick(View v) {
                    Intent it = new Intent(ScheduleActivity.this, DektActivity.class);
                    it.putExtra(DektActivity.EXTRA_URL, url);
                    it.putExtra(DektActivity.EXTRA_AUTO_ENROLL, true);
                    startActivity(it);
                }
            });
            links.addView(signup);
            col.addView(links);
        }

        row.addView(col, new LinearLayout.LayoutParams(0,
                ViewGroup.LayoutParams.WRAP_CONTENT, 1f));
        row.setOnClickListener(new View.OnClickListener() {
            @Override
            public void onClick(View v) {
                edit(o);
            }
        });
        return row;
    }

    private String remindText(int v) {
        if (v < 0) return "不提醒";
        if (v == 0) return "准时提醒";
        if (v % 1440 == 0) return "提前 " + (v / 1440) + " 天";
        if (v % 60 == 0) return "提前 " + (v / 60) + " 小时";
        return "提前 " + v + " 分钟";
    }

    private void toggle(final long id, final boolean done) {
        final Context ctx = this;
        new Thread(new Runnable() {
            @Override
            public void run() {
                final boolean ok = RustCore.scheduleToggle(ctx, id, done);
                ReminderScheduler.reschedule(ctx);
                ui.post(new Runnable() {
                    @Override
                    public void run() {
                        if (!ok) UiKit.toast(ScheduleActivity.this, "更新失败");
                        load();
                    }
                });
            }
        }, "lancai-schedule-toggle").start();
    }

    // ------------------------------------------------------------------
    // 新建 / 编辑
    // ------------------------------------------------------------------

    private EditText input(String hint) {
        EditText e = new EditText(this);
        e.setHint(hint);
        e.setTextSize(14);
        e.setTextColor(UiKit.TEXT);
        e.setHintTextColor(0xFF9AA8A2);
        LinearLayout.LayoutParams lp = new LinearLayout.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.WRAP_CONTENT);
        lp.topMargin = dp(4);
        e.setLayoutParams(lp);
        return e;
    }

    private TextView pickRow() {
        TextView t = UiKit.text(this, "", 14, UiKit.TEXT);
        t.setPadding(dp(12), dp(11), dp(12), dp(11));
        LinearLayout.LayoutParams lp = new LinearLayout.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.WRAP_CONTENT);
        lp.topMargin = dp(4);
        t.setLayoutParams(lp);
        return t;
    }

    private void updateWhenRows(TextView dateRow, TextView timeRow, long ms) {
        if (ms <= 0) {
            dateRow.setText("点此选择日期");
            timeRow.setText("点此选择时间");
            dateRow.setTextColor(0xFF9AA8A2);
            timeRow.setTextColor(0xFF9AA8A2);
            return;
        }
        Calendar c = Calendar.getInstance();
        c.setTimeInMillis(ms);
        dateRow.setText(String.format(Locale.CHINA, "%d月%d日 周%s",
                c.get(Calendar.MONTH) + 1, c.get(Calendar.DAY_OF_MONTH),
                "一二三四五六日".charAt((c.get(Calendar.DAY_OF_WEEK) + 5) % 7)));
        timeRow.setText(String.format(Locale.CHINA, "%02d:%02d",
                c.get(Calendar.HOUR_OF_DAY), c.get(Calendar.MINUTE)));
        dateRow.setTextColor(UiKit.TEXT);
        timeRow.setTextColor(UiKit.TEXT);
    }

    private String whenString(long ms) {
        if (ms <= 0) return "";
        return new SimpleDateFormat("yyyy-MM-dd HH:mm", Locale.CHINA).format(new Date(ms));
    }

    private void edit(final JSONObject item) {
        final boolean editing = item != null;
        LinearLayout form = UiKit.vertical(this);
        form.setPadding(dp(18), dp(4), dp(18), dp(4));

        final EditText etTitle = input("标题，例如：报名志愿服务 / 高数讲座");
        final EditText etNote = input("备注（可选）");
        etNote.setMinLines(2);
        etNote.setMaxLines(4);
        etNote.setGravity(Gravity.TOP | Gravity.START);
        final EditText etUrl = input("活动链接（可选，第二课堂可直接打开）");
        etUrl.setInputType(InputType.TYPE_TEXT_VARIATION_URI);

        final long[] picked = {0};
        final TextView dateRow = pickRow();
        final TextView timeRow = pickRow();
        final TextView clearTime = UiKit.text(this, "不设时间（只进列表，不上课表）",
                12, 0xFF5B5BD6);
        clearTime.setPadding(0, dp(8), 0, dp(2));

        dateRow.setOnClickListener(new View.OnClickListener() {
            @Override
            public void onClick(View v) {
                Calendar c = Calendar.getInstance();
                if (picked[0] > 0) c.setTimeInMillis(picked[0]);
                new DatePickerDialog(ScheduleActivity.this,
                        new DatePickerDialog.OnDateSetListener() {
                            @Override
                            public void onDateSet(DatePicker view, int year, int month, int day) {
                                Calendar c2 = Calendar.getInstance();
                                if (picked[0] > 0) c2.setTimeInMillis(picked[0]);
                                c2.set(Calendar.YEAR, year);
                                c2.set(Calendar.MONTH, month);
                                c2.set(Calendar.DAY_OF_MONTH, day);
                                picked[0] = c2.getTimeInMillis();
                                updateWhenRows(dateRow, timeRow, picked[0]);
                            }
                        }, c.get(Calendar.YEAR), c.get(Calendar.MONTH),
                        c.get(Calendar.DAY_OF_MONTH)).show();
            }
        });
        timeRow.setOnClickListener(new View.OnClickListener() {
            @Override
            public void onClick(View v) {
                Calendar c = Calendar.getInstance();
                if (picked[0] > 0) c.setTimeInMillis(picked[0]);
                new TimePickerDialog(ScheduleActivity.this,
                        new TimePickerDialog.OnTimeSetListener() {
                            @Override
                            public void onTimeSet(TimePicker view, int hour, int minute) {
                                Calendar c2 = Calendar.getInstance();
                                if (picked[0] > 0) c2.setTimeInMillis(picked[0]);
                                c2.set(Calendar.HOUR_OF_DAY, hour);
                                c2.set(Calendar.MINUTE, minute);
                                picked[0] = c2.getTimeInMillis();
                                updateWhenRows(dateRow, timeRow, picked[0]);
                            }
                        }, c.get(Calendar.HOUR_OF_DAY), c.get(Calendar.MINUTE), true).show();
            }
        });
        clearTime.setOnClickListener(new View.OnClickListener() {
            @Override
            public void onClick(View v) {
                picked[0] = 0;
                updateWhenRows(dateRow, timeRow, 0);
            }
        });
        updateWhenRows(dateRow, timeRow, 0);

        form.addView(UiKit.label(this, "标题"));
        form.addView(etTitle);
        form.addView(UiKit.label(this, "日期"));
        form.addView(dateRow);
        form.addView(UiKit.label(this, "开始时间"));
        form.addView(timeRow);
        form.addView(clearTime);
        form.addView(UiKit.label(this, "类型"));
        final Spinner spKind = new Spinner(this);
        spKind.setAdapter(new ArrayAdapter<>(this,
                android.R.layout.simple_spinner_item, KIND_NAMES));
        form.addView(spKind);
        form.addView(UiKit.label(this, "提前提醒"));
        final Spinner spRemind = new Spinner(this);
        spRemind.setAdapter(new ArrayAdapter<>(this,
                android.R.layout.simple_spinner_item, REMIND_NAMES));
        form.addView(spRemind);
        form.addView(UiKit.label(this, "备注"));
        form.addView(etNote);
        form.addView(UiKit.label(this, "活动链接"));
        form.addView(etUrl);

        final long id;
        final boolean done;
        if (editing) {
            id = item.optLong("id", 0);
            done = item.optBoolean("done", false);
            etTitle.setText(item.optString("title", ""));
            long due = item.optLong("due_ms", 0);
            picked[0] = due;
            updateWhenRows(dateRow, timeRow, due);
            etNote.setText(item.optString("note", ""));
            etUrl.setText(item.optString("url", ""));
            spKind.setSelection("activity".equals(item.optString("kind", "todo")) ? 1 : 0);
            int rm = item.optInt("remind_min", 10);
            int idx = 3;
            for (int i = 0; i < REMIND_VALUES.length; i++) {
                if (REMIND_VALUES[i] == rm) {
                    idx = i;
                    break;
                }
            }
            spRemind.setSelection(idx);
        } else {
            id = 0;
            done = false;
        }

        ScrollView scroll = new ScrollView(this);
        scroll.addView(form, new ViewGroup.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.WRAP_CONTENT));

        AlertDialog.Builder b = new AlertDialog.Builder(this)
                .setTitle(editing ? "编辑日程" : "新建日程")
                .setView(scroll)
                .setPositiveButton("保存", null)
                .setNegativeButton("取消", null);
        if (editing) {
            b.setNeutralButton("删除", new DialogInterface.OnClickListener() {
                @Override
                public void onClick(DialogInterface d, int which) {
                    confirmDelete(id);
                }
            });
        }
        final AlertDialog dialog = b.create();
        dialog.setOnShowListener(new DialogInterface.OnShowListener() {
            @Override
            public void onShow(DialogInterface d) {
                Button save = dialog.getButton(DialogInterface.BUTTON_POSITIVE);
                save.setOnClickListener(new View.OnClickListener() {
                    @Override
                    public void onClick(View v) {
                        String title = etTitle.getText().toString().trim();
                        if (title.isEmpty()) {
                            UiKit.toast(ScheduleActivity.this, "先填一个标题");
                            etTitle.requestFocus();
                            return;
                        }
                        try {
                            JSONObject o = new JSONObject();
                            o.put("id", id);
                            o.put("done", done);
                            o.put("title", title);
                            o.put("due", whenString(picked[0]));
                            o.put("note", etNote.getText().toString().trim());
                            o.put("kind", KIND_IDS[spKind.getSelectedItemPosition()]);
                            o.put("url", etUrl.getText().toString().trim());
                            o.put("remind_min",
                                    REMIND_VALUES[spRemind.getSelectedItemPosition()]);
                            if (editing) {
                                o.put("created_ms", item.optLong("created_ms", 0));
                            }
                            saveChecked(o, dialog);
                        } catch (Throwable t) {
                            UiKit.toastLong(ScheduleActivity.this, "保存失败");
                        }
                    }
                });
            }
        });
        dialog.show();
    }

    /**
     * 保存前先判断能否参加：活动带链接时，用链接所在文本与课表比较，
     * 再询问用户是否开启自动报名。普通待办直接保存。
     */
    private void saveChecked(final JSONObject o, final AlertDialog editDialog) {
        final String kind = o.optString("kind", "todo");
        final String url = o.optString("url", "").trim();
        if (!"activity".equals(kind) || url.isEmpty() || !RustCore.available()) {
            editDialog.dismiss();
            save(o);
            return;
        }
        editDialog.dismiss();
        final Context ctx = this;
        final String text = o.optString("title", "") + "\n"
                + o.optString("note", "") + "\n" + o.optString("due", "");
        new Thread(new Runnable() {
            @Override
            public void run() {
                final String json = RustCore.enrollCheck(ctx, text);
                ui.post(new Runnable() {
                    @Override
                    public void run() {
                        askAutoSignup(o, json);
                    }
                });
            }
        }, "lancai-enroll-check").start();
    }

    private void askAutoSignup(final JSONObject o, String json) {
        boolean canAttend = true;
        boolean timeKnown = false;
        long eventStart = 0;
        long deadline = 0;
        String reason = "没有识别到具体时间，可随时点「一键报名」";
        if (json != null) {
            try {
                JSONObject a = new JSONObject(json);
                canAttend = a.optBoolean("can_attend", true);
                timeKnown = a.optBoolean("time_known", false);
                eventStart = a.optLong("event_start_ms", 0);
                deadline = a.optLong("deadline_ms", 0);
                reason = a.optString("reason", reason);
            } catch (Throwable ignore) {
            }
        }

        StringBuilder msg = new StringBuilder();
        msg.append("能否参加：").append(canAttend ? "可以参加" : "可能无法参加").append('\n');
        msg.append(reason);
        msg.append("\n\n自动报名会在开始/截止前发通知，打开报名页后自动填写姓名、学号、手机号。");
        msg.append("\n（个人信息可在设置里填写）");
        if (!timeKnown) {
            msg.append("\n\n未识别到时间：只保留开关，不会自动提醒。");
        }

        final long baseDeadline = deadline;
        final long baseStart = eventStart;

        new AlertDialog.Builder(this).setTitle("是否自动报名？")
                .setMessage(msg.toString())
                .setPositiveButton("自动报名", new DialogInterface.OnClickListener() {
                    @Override
                    public void onClick(DialogInterface d, int which) {
                        long at = 0;
                        long now = System.currentTimeMillis();
                        long base = baseDeadline > 0 ? baseDeadline : baseStart;
                        if (base > now) {
                            at = base - 10 * 60 * 1000L;
                            if (at <= now) at = now + 60 * 1000L;
                        }
                        try {
                            o.put("auto_signup", true);
                            o.put("signup_at_ms", at);
                        } catch (Throwable ignore) {
                        }
                        save(o);
                        if (at > 0) {
                            UiKit.toastLong(ScheduleActivity.this,
                                    "已开启自动报名，到点会通知并打开报名页");
                        } else {
                            UiKit.toastLong(ScheduleActivity.this,
                                    "已开启自动报名；未识别到时间，可点一键报名");
                        }
                    }
                })
                .setNeutralButton("仅提醒", new DialogInterface.OnClickListener() {
                    @Override
                    public void onClick(DialogInterface d, int which) {
                        try {
                            o.put("auto_signup", false);
                            o.put("signup_at_ms", 0);
                        } catch (Throwable ignore) {
                        }
                        save(o);
                    }
                })
                .setNegativeButton("取消", null)
                .show();
    }

    private void save(final JSONObject item) {
        final Context ctx = this;
        new Thread(new Runnable() {
            @Override
            public void run() {
                final String json = RustCore.scheduleSave(ctx, item.toString());
                ReminderScheduler.reschedule(ctx);
                ui.post(new Runnable() {
                    @Override
                    public void run() {
                        try {
                            JSONObject o = new JSONObject(json == null ? "{}" : json);
                            if (!o.optBoolean("ok", false)) {
                                UiKit.toastLong(ScheduleActivity.this,
                                        o.optString("error", "保存失败"));
                                return;
                            }
                        } catch (Throwable t) {
                            UiKit.toastLong(ScheduleActivity.this, "保存返回异常");
                            return;
                        }
                        UiKit.toast(ScheduleActivity.this, "已保存");
                        load();
                    }
                });
            }
        }, "lancai-schedule-save").start();
    }

    private void confirmDelete(final long id) {
        UiKit.confirm(this, "删除这条日程", "删除后不可恢复。", "删除", new Runnable() {
            @Override
            public void run() {
                final Context ctx = ScheduleActivity.this;
                new Thread(new Runnable() {
                    @Override
                    public void run() {
                        final boolean ok = RustCore.scheduleDelete(ctx, id);
                        ReminderScheduler.reschedule(ctx);
                        ui.post(new Runnable() {
                            @Override
                            public void run() {
                                if (!ok) UiKit.toast(ScheduleActivity.this, "删除失败");
                                load();
                            }
                        });
                    }
                }, "lancai-schedule-delete").start();
            }
        });
    }

    private void confirmClearDone() {
        UiKit.confirm(this, "清除已完成", "会删除所有已勾选的日程，不可恢复。", "清除",
                new Runnable() {
                    @Override
                    public void run() {
                        final Context ctx = ScheduleActivity.this;
                        new Thread(new Runnable() {
                            @Override
                            public void run() {
                                final int n = RustCore.scheduleClearDone(ctx);
                                ReminderScheduler.reschedule(ctx);
                                ui.post(new Runnable() {
                                    @Override
                                    public void run() {
                                        UiKit.toast(ScheduleActivity.this,
                                                n > 0 ? "已清除 " + n + " 条" : "没有已完成的日程");
                                        load();
                                    }
                                });
                            }
                        }, "lancai-schedule-clear").start();
                    }
                });
    }

    @Override
    public void onRequestPermissionsResult(int requestCode, String[] permissions,
                                           int[] grantResults) {
        super.onRequestPermissionsResult(requestCode, permissions, grantResults);
        if (requestCode == NotificationHelper.PERMISSION_REQ) {
            ToastCompat.show(this, NotificationHelper.canNotify(this)
                    ? "通知已开启" : "通知未开启，可稍后在系统设置中开启");
            load();
        }
    }

    private int dp(float v) {
        return UiKit.dp(this, v);
    }

    /** 避免额外依赖的小工具。 */
    private static final class ToastCompat {
        static void show(Context c, String s) {
            UiKit.toastLong(c, s);
        }
    }
}
