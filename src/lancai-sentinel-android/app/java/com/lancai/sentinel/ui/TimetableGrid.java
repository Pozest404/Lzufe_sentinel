package com.lancai.sentinel.ui;

// [OWNER: UI] 周视图网格：同一格里同时画课程（按节次跨行）和日程/活动（按时间定位）。
// 被 MainActivity（首页课表）和 TimetableActivity（完整课表）复用。
// 视觉参考「星课程表」：衬线标题、日期表头、柔和色块、竖排课程名。

import android.content.Context;
import android.graphics.Canvas;
import android.graphics.Color;
import android.graphics.Paint;
import android.graphics.Typeface;
import android.text.TextUtils;
import android.view.Gravity;
import android.view.View;
import android.view.ViewGroup;
import android.widget.FrameLayout;
import android.widget.LinearLayout;
import android.widget.TextView;

import org.json.JSONArray;
import org.json.JSONObject;

import java.text.SimpleDateFormat;
import java.util.Calendar;
import java.util.Date;
import java.util.Locale;

public final class TimetableGrid {

    public static final int ROW_H = 54;
    /** 一天 10 节：上午 4 节、下午 4 节、晚上 2 节。 */
    public static final int SECTION_COUNT = 10;
    public static final int ACTIVITY_MIN_HEIGHT = 30;

    private static final int LINE = 0xFFEFEFF4;
    private static final int INK = 0xFF1E1E24;
    private static final int MUTED = 0xFF8E8E9A;
    private static final int INDIGO = 0xFF5B5BD6;
    private static final int INDIGO_SOFT = 0xFFEAEAFB;
    private static final int TODAY_BG = 0xFFF7F7FE;
    private static final int DOT_RED = 0xFFE5484D;

    private static final int[] STARTS = {
            8 * 60 + 30, 9 * 60 + 25, 10 * 60 + 40, 11 * 60 + 35,
            14 * 60 + 30, 15 * 60 + 25, 16 * 60 + 30, 17 * 60 + 25,
            19 * 60, 19 * 60 + 55
    };
    private static final int[] ENDS = {
            9 * 60 + 25, 10 * 60 + 20, 11 * 60 + 35, 12 * 60 + 30,
            15 * 60 + 25, 16 * 60 + 20, 17 * 60 + 25, 18 * 60 + 20,
            19 * 60 + 55, 20 * 60 + 50
    };

    // 柔和底色 + 深一点的字色，一组一组轮换。
    private static final int[] BLOCK_BG = {
            0xFFF4CBCB, 0xFFEADFC4, 0xFFCBDAF3, 0xFFDCCBEE, 0xFFD0E4CA, 0xFFF3D5C4
    };
    private static final int[] BLOCK_FG = {
            0xFF8D4E4E, 0xFF8A6C3C, 0xFF44618F, 0xFF6A4C93, 0xFF4C6D47, 0xFF8D5B41
    };
    private static final int ACTIVITY_BG = 0xFFF7DCC8;
    private static final int ACTIVITY_FG = 0xFF8E5330;

    private static final int SEC_COL_DP = 40;
    private static final int HEADER_H = 42;
    private static final String[] DAY_NAMES = {"一", "二", "三", "四", "五", "六", "日"};

    public interface Listener {
        void onCourseClick(JSONObject course);

        void onActivityClick(JSONObject todo);
    }

    private TimetableGrid() {
    }

    /** 构建：日期表头 + 分隔线 + 网格体（课程 + 活动叠加）。 */
    public static LinearLayout build(Context ctx, JSONObject data, JSONArray schedule,
                                     int displayWeek, Listener l) {
        LinearLayout wrap = UiKit.vertical(ctx);
        JSONArray courses = data == null ? null : data.optJSONArray("courses");
        int total = data == null ? 20 : Math.max(1, data.optInt("weeks", 20));
        displayWeek = Math.max(1, Math.min(displayWeek, total));
        int currentWeek = data == null ? 1 : Math.max(1, data.optInt("current_week", 1));

        final int days = daysWithContent(courses, schedule);

        wrap.addView(dayHeader(ctx, days, data, displayWeek, currentWeek));
        wrap.addView(UiKit.divider(ctx));
        wrap.addView(gridBody(ctx, courses, schedule, days, data, displayWeek, currentWeek, l));
        return wrap;
    }

    private static int daysWithContent(JSONArray courses, JSONArray schedule) {
        boolean weekend = false;
        if (courses != null) {
            for (int i = 0; i < courses.length(); i++) {
                JSONObject c = courses.optJSONObject(i);
                if (c != null && c.optInt("day", 0) >= 6) {
                    weekend = true;
                    break;
                }
            }
        }
        if (!weekend && schedule != null) {
            for (int i = 0; i < schedule.length(); i++) {
                JSONObject t = schedule.optJSONObject(i);
                if (t == null || t.optBoolean("done", false) || t.optLong("due_ms", 0) <= 0) {
                    continue;
                }
                int wd = weekdayOf(t.optLong("due_ms", 0));
                if (wd >= 6) {
                    weekend = true;
                    break;
                }
            }
        }
        return weekend ? 7 : 5;
    }

    private static LinearLayout dayHeader(Context ctx, int days, JSONObject data,
                                          int displayWeek, int currentWeek) {
        LinearLayout row = UiKit.horizontal(ctx);
        row.setGravity(Gravity.BOTTOM);
        row.setPadding(0, dp(ctx, 2), 0, dp(ctx, 2));

        LinearLayout monthBox = UiKit.vertical(ctx);
        monthBox.setGravity(Gravity.CENTER);
        TextView month = serif(ctx, monthLabel(data, displayWeek), 12.5f, INK);
        month.setGravity(Gravity.CENTER);
        monthBox.addView(month);
        row.addView(monthBox, new LinearLayout.LayoutParams(dp(ctx, SEC_COL_DP),
                dp(ctx, HEADER_H)));

        int today = todayWeekday();
        boolean todayVisible = displayWeek == currentWeek;
        for (int d = 1; d <= days; d++) {
            boolean isToday = todayVisible && today == d;
            LinearLayout box = UiKit.vertical(ctx);
            box.setGravity(Gravity.CENTER_HORIZONTAL);

            TextView name = serif(ctx, "周" + DAY_NAMES[d - 1], 9.5f, MUTED);
            name.setGravity(Gravity.CENTER);
            box.addView(name);

            String num = dateNumber(data, displayWeek, d);
            boolean todayMark = isToday && !num.isEmpty();
            FrameLayout dateWrap = new FrameLayout(ctx);
            TextView dt = serif(ctx, num, 15f, todayMark ? Color.WHITE : INK);
            dt.setGravity(Gravity.CENTER);
            if (todayMark) {
                UiKit.rounded(dt, ctx, INDIGO, 0, 13);
                dt.setTypeface(Typeface.create(Typeface.SERIF, Typeface.BOLD));
            }
            FrameLayout.LayoutParams dlp = new FrameLayout.LayoutParams(
                    dp(ctx, 24), dp(ctx, 24), Gravity.CENTER);
            dateWrap.addView(dt, dlp);
            box.addView(dateWrap, new LinearLayout.LayoutParams(
                    ViewGroup.LayoutParams.MATCH_PARENT, dp(ctx, 25)));

            if (todayMark) {
                View dot = new View(ctx);
                UiKit.rounded(dot, ctx, DOT_RED, 0, 3);
                LinearLayout.LayoutParams dl = new LinearLayout.LayoutParams(
                        dp(ctx, 3), dp(ctx, 3));
                box.addView(dot, dl);
            }
            LinearLayout.LayoutParams lp = new LinearLayout.LayoutParams(0,
                    ViewGroup.LayoutParams.WRAP_CONTENT, 1f);
            lp.setMargins(dp(ctx, 1), 0, dp(ctx, 1), 0);
            row.addView(box, lp);
        }
        return row;
    }

    private static LinearLayout gridBody(Context ctx, JSONArray courses, JSONArray schedule,
                                         int days, JSONObject data, int displayWeek,
                                         int currentWeek, final Listener l) {
        LinearLayout body = UiKit.horizontal(ctx);
        body.setGravity(Gravity.TOP);

        LinearLayout secCol = UiKit.vertical(ctx);
        for (int sec = 1; sec <= SECTION_COUNT; sec++) {
            LinearLayout box = UiKit.vertical(ctx);
            box.setGravity(Gravity.CENTER);
            TextView no = serif(ctx, String.valueOf(sec), 12.5f, INK);
            no.setGravity(Gravity.CENTER);
            TextView start = UiKit.text(ctx, hhmmOf(STARTS[sec - 1]), 8, MUTED);
            start.setGravity(Gravity.CENTER);
            TextView end = UiKit.text(ctx, hhmmOf(ENDS[sec - 1]), 8, MUTED);
            end.setGravity(Gravity.CENTER);
            box.addView(no);
            box.addView(start);
            box.addView(end);
            secCol.addView(box, new LinearLayout.LayoutParams(dp(ctx, SEC_COL_DP),
                    dp(ctx, ROW_H)));
        }
        body.addView(secCol, new LinearLayout.LayoutParams(dp(ctx, SEC_COL_DP),
                dp(ctx, ROW_H * SECTION_COUNT)));

        int today = todayWeekday();
        boolean todayVisible = displayWeek == currentWeek;
        for (int d = 1; d <= days; d++) {
            FrameLayout dayFrame = new FrameLayout(ctx);
            if (todayVisible && today == d) dayFrame.setBackgroundColor(TODAY_BG);
            dayFrame.addView(new LinesView(ctx), new FrameLayout.LayoutParams(
                    ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.MATCH_PARENT));

            LinearLayout courseCol = buildCourseColumn(ctx, courses, displayWeek, d, l);
            dayFrame.addView(courseCol, new FrameLayout.LayoutParams(
                    ViewGroup.LayoutParams.MATCH_PARENT, dp(ctx, ROW_H * SECTION_COUNT)));
            addActivities(ctx, dayFrame, schedule, d, data, displayWeek, l);

            LinearLayout.LayoutParams lp = new LinearLayout.LayoutParams(0,
                    dp(ctx, ROW_H * SECTION_COUNT), 1f);
            lp.setMargins(dp(ctx, 1), 0, dp(ctx, 1), 0);
            body.addView(dayFrame, lp);
        }
        return body;
    }

    private static LinearLayout buildCourseColumn(Context ctx, JSONArray courses,
                                                  int displayWeek, int day, final Listener l) {
        LinearLayout col = UiKit.vertical(ctx);
        int sec = 1;
        while (sec <= SECTION_COUNT) {
            JSONObject c = findCourseAt(courses, day, sec, displayWeek);
            if (c == null) {
                col.addView(new View(ctx),
                        new LinearLayout.LayoutParams(ViewGroup.LayoutParams.MATCH_PARENT, dp(ctx, ROW_H)));
                sec++;
                continue;
            }
            int s = Math.max(1, c.optInt("start_section", sec));
            int e = Math.max(s, c.optInt("end_section", s));
            int span = Math.min(e, SECTION_COUNT) - s + 1;
            LinearLayout.LayoutParams lp = new LinearLayout.LayoutParams(
                    ViewGroup.LayoutParams.MATCH_PARENT, dp(ctx, ROW_H * span));
            lp.setMargins(dp(ctx, 2), dp(ctx, 2), dp(ctx, 2), dp(ctx, 1));
            col.addView(courseBlock(ctx, c, span, l), lp);
            sec = Math.min(e, SECTION_COUNT) + 1;
        }
        return col;
    }

    private static View courseBlock(Context ctx, final JSONObject c, int span, final Listener l) {
        int idx = (c.optString("name", "").hashCode() & 0x7fffffff) % BLOCK_BG.length;
        LinearLayout box = UiKit.vertical(ctx);
        UiKit.rounded(box, ctx, BLOCK_BG[idx], 0, 12);
        box.setPadding(dp(ctx, 4), dp(ctx, 4), dp(ctx, 4), dp(ctx, 4));

        String nameRaw = c.optString("name", "");
        String loc = c.optString("location", "");
        String teacher = c.optString("teacher", "");
        String where = loc;
        if (!teacher.isEmpty()) where = where.isEmpty() ? teacher : where + " " + teacher;
        boolean hasLoc = !where.isEmpty();

        box.setGravity(hasLoc ? (Gravity.TOP | Gravity.CENTER_HORIZONTAL)
                : Gravity.CENTER);

        TextView name = serif(ctx, nameRaw, span >= 2 ? 9.5f : 9f, BLOCK_FG[idx]);
        name.setGravity(Gravity.CENTER);
        name.setMaxLines(span >= 2 ? 3 : 2);
        name.setEllipsize(TextUtils.TruncateAt.END);
        name.setLineSpacing(0, 1.0f);
        box.addView(name);

        if (hasLoc) {
            TextView t = UiKit.text(ctx, "@ " + where, 7.5f, BLOCK_FG[idx]);
            t.setAlpha(0.8f);
            t.setMaxLines(span >= 2 ? 2 : 1);
            t.setEllipsize(TextUtils.TruncateAt.END);
            t.setGravity(Gravity.CENTER);
            t.setLineSpacing(0, 0.95f);
            t.setPadding(0, dp(ctx, 2), 0, 0);
            box.addView(t);
        }
        if (l != null) {
            box.setOnClickListener(new View.OnClickListener() {
                @Override
                public void onClick(View v) {
                    try {
                        l.onCourseClick(c);
                    } catch (Throwable ignore) {
                    }
                }
            });
        }
        return box;
    }

    private static void addActivities(Context ctx, FrameLayout dayFrame, JSONArray schedule,
                                      int day, JSONObject data, int displayWeek, final Listener l) {
        if (schedule == null) return;
        Date colDate = columnDate(data, displayWeek, day);
        for (int i = 0; i < schedule.length(); i++) {
            JSONObject t = schedule.optJSONObject(i);
            if (t == null) continue;
            long due = t.optLong("due_ms", 0);
            if (due <= 0 || t.optBoolean("done", false)) continue;
            if (weekdayOf(due) != day) continue;
            if (colDate != null && !sameDay(due, colDate)) continue;

            int minutes = minuteOf(due);
            int y = yForTime(ctx, minutes);
            int height = Math.max(dp(ctx, ACTIVITY_MIN_HEIGHT), (int) (dp(ctx, ROW_H) * 0.75));
            if (y + height > SECTION_COUNT * dp(ctx, ROW_H)) {
                height = SECTION_COUNT * dp(ctx, ROW_H) - y;
            }
            if (height <= 0) continue;

            View block = activityBlock(ctx, t, due, l);
            FrameLayout.LayoutParams lp = new FrameLayout.LayoutParams(
                    ViewGroup.LayoutParams.MATCH_PARENT, height);
            lp.topMargin = y;
            lp.leftMargin = dp(ctx, 2);
            lp.rightMargin = dp(ctx, 2);
            dayFrame.addView(block, lp);
        }
    }

    private static View activityBlock(Context ctx, final JSONObject t, long due, final Listener l) {
        LinearLayout box = UiKit.vertical(ctx);
        UiKit.rounded(box, ctx, ACTIVITY_BG, 0, 10);
        box.setPadding(dp(ctx, 4), dp(ctx, 3), dp(ctx, 4), dp(ctx, 3));
        String title = t.optString("title", "");
        if (TextUtils.isEmpty(title)) title = "活动";
        TextView name = serif(ctx, title, 9f, ACTIVITY_FG);
        name.setTypeface(Typeface.create(Typeface.SERIF, Typeface.BOLD));
        name.setMaxLines(2);
        name.setEllipsize(TextUtils.TruncateAt.END);
        box.addView(name);
        TextView tm = UiKit.text(ctx, UiKit.hhmm(due), 7.5f, ACTIVITY_FG);
        tm.setAlpha(0.8f);
        tm.setMaxLines(1);
        box.addView(tm);
        if (l != null) {
            box.setOnClickListener(new View.OnClickListener() {
                @Override
                public void onClick(View v) {
                    try {
                        l.onActivityClick(t);
                    } catch (Throwable ignore) {
                    }
                }
            });
        }
        return box;
    }

    private static JSONObject findCourseAt(JSONArray courses, int day, int sec, int displayWeek) {
        if (courses == null) return null;
        for (int i = 0; i < courses.length(); i++) {
            JSONObject c = courses.optJSONObject(i);
            if (c == null) continue;
            if (c.optInt("day", 0) != day) continue;
            if (c.optInt("start_section", 0) != sec) continue;
            if (!occursIn(c, displayWeek)) continue;
            return c;
        }
        return null;
    }

    private static boolean occursIn(JSONObject c, int week) {
        JSONArray ws = c.optJSONArray("weeks");
        if (ws == null || ws.length() == 0) return true;
        for (int i = 0; i < ws.length(); i++) {
            if (ws.optInt(i, 0) == week) return true;
        }
        return false;
    }

    private static int yForTime(Context ctx, int minutes) {
        if (minutes <= STARTS[0]) return 0;
        int rowPx = dp(ctx, ROW_H);
        for (int i = 0; i < SECTION_COUNT; i++) {
            int s = STARTS[i], e = ENDS[i];
            if (minutes < s) return i * rowPx;
            if (minutes < e) {
                return i * rowPx + (int) ((minutes - s) / (double) (e - s) * rowPx);
            }
        }
        return SECTION_COUNT * rowPx;
    }

    private static final ThreadLocal<SimpleDateFormat> F_YMD = new ThreadLocal<SimpleDateFormat>() {
        @Override
        protected SimpleDateFormat initialValue() {
            return new SimpleDateFormat("yyyy-MM-dd", Locale.CHINA);
        }
    };
    private static final ThreadLocal<SimpleDateFormat> F_DAY = new ThreadLocal<SimpleDateFormat>() {
        @Override
        protected SimpleDateFormat initialValue() {
            return new SimpleDateFormat("d", Locale.CHINA);
        }
    };
    private static final ThreadLocal<SimpleDateFormat> F_MONTH = new ThreadLocal<SimpleDateFormat>() {
        @Override
        protected SimpleDateFormat initialValue() {
            return new SimpleDateFormat("M月", Locale.CHINA);
        }
    };
    private static final ThreadLocal<Calendar> CAL = new ThreadLocal<Calendar>() {
        @Override
        protected Calendar initialValue() {
            return Calendar.getInstance();
        }
    };

    private static Date columnDate(JSONObject data, int displayWeek, int day) {
        if (data == null) return null;
        String start = data.optString("term_start", "");
        if (start.isEmpty()) return null;
        try {
            Date d = F_YMD.get().parse(start);
            if (d == null) return null;
            Calendar c = CAL.get();
            c.setTime(d);
            c.add(Calendar.DAY_OF_YEAR, (displayWeek - 1) * 7 + (day - 1));
            return c.getTime();
        } catch (Throwable t) {
            return null;
        }
    }

    private static boolean sameDay(long ms, Date date) {
        Calendar a = CAL.get();
        a.setTimeInMillis(ms);
        int y1 = a.get(Calendar.YEAR);
        int d1 = a.get(Calendar.DAY_OF_YEAR);
        a.setTime(date);
        return y1 == a.get(Calendar.YEAR) && d1 == a.get(Calendar.DAY_OF_YEAR);
    }

    private static String dateNumber(JSONObject data, int displayWeek, int day) {
        Date d = columnDate(data, displayWeek, day);
        if (d == null) return "";
        return F_DAY.get().format(d);
    }

    private static String monthLabel(JSONObject data, int displayWeek) {
        Date d = columnDate(data, displayWeek, 1);
        if (d == null) return "";
        return F_MONTH.get().format(d);
    }

    private static int weekdayOf(long ms) {
        Calendar c = CAL.get();
        c.setTimeInMillis(ms);
        int v = c.get(Calendar.DAY_OF_WEEK);
        return v == Calendar.SUNDAY ? 7 : v - 1;
    }

    private static int minuteOf(long ms) {
        Calendar c = CAL.get();
        c.setTimeInMillis(ms);
        return c.get(Calendar.HOUR_OF_DAY) * 60 + c.get(Calendar.MINUTE);
    }

    private static int todayWeekday() {
        int v = Calendar.getInstance().get(Calendar.DAY_OF_WEEK);
        return v == Calendar.SUNDAY ? 7 : v - 1;
    }

    private static String hhmmOf(int minutes) {
        return String.format(Locale.CHINA, "%02d:%02d", minutes / 60, minutes % 60);
    }

    /** 竖排文字：每个字一行，超过 max 行用省略号收尾。 */
    private static String vertical(String s, int max) {
        if (s == null) return "";
        StringBuilder b = new StringBuilder();
        int n = 0;
        for (int i = 0; i < s.length(); i++) {
            char ch = s.charAt(i);
            if (Character.isWhitespace(ch)) continue;
            if (n >= max) {
                b.append('…');
                break;
            }
            if (n > 0) b.append('\n');
            b.append(ch);
            n++;
        }
        return b.toString();
    }

    private static TextView serif(Context ctx, String s, float sp, int color) {
        TextView t = UiKit.text(ctx, s, sp, color);
        t.setTypeface(Typeface.create(Typeface.SERIF, Typeface.NORMAL));
        return t;
    }

    private static int dp(Context ctx, float v) {
        return UiKit.dp(ctx, v);
    }

    /** 网格线：每节一条横线，左侧一条竖线。 */
    private static final class LinesView extends View {
        private final Paint paint = new Paint();
        private final int rowPx;

        LinesView(Context c) {
            super(c);
            paint.setColor(LINE);
            paint.setStrokeWidth(Math.max(1, UiKit.dp(c, 0.8f)));
            rowPx = UiKit.dp(c, ROW_H);
        }

        @Override
        protected void onDraw(Canvas canvas) {
            int w = getWidth();
            int h = getHeight();
            for (int y = rowPx; y <= h; y += rowPx) {
                canvas.drawLine(0, y, w, y, paint);
            }
            canvas.drawLine(0, 0, 0, h, paint);
        }
    }
}
