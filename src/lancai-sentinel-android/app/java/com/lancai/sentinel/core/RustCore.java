package com.lancai.sentinel.core;

// [OWNER: LEAD] JNI 薄壳：只负责加载 Rust 库并传配置/文本。
// 规则打分、存储、AI 总结、PushPlus 推送都在 Rust 侧（src/android.rs）。

import android.content.Context;
import android.util.Log;

public final class RustCore {

    private static final String TAG = "LCS";
    private static final boolean LOADED;

    static {
        boolean ok;
        try {
            System.loadLibrary("lancai_sentinel");
            ok = true;
        } catch (Throwable t) {
            Log.e(TAG, "Rust 核心加载失败", t);
            ok = false;
        }
        LOADED = ok;
    }

    private RustCore() {
    }

    public static boolean available() {
        return LOADED;
    }

    public static String version() {
        if (!LOADED) return "Rust 核心未加载";
        try {
            return nativeVersion();
        } catch (Throwable t) {
            return "Rust 核心异常：" + t.getMessage();
        }
    }

    /** 分析文本，返回 MessageOutcome JSON；失败返回 null。 */
    public static String analyze(Context c, String source, String text, boolean persist) {
        if (!LOADED) return null;
        try {
            return nativeAnalyze(Config.get(c).toJson(), Config.dataDir(c),
                    source == null ? "" : source, text, persist);
        } catch (Throwable t) {
            Log.e(TAG, "nativeAnalyze 失败", t);
            return null;
        }
    }

    /** 最近消息 JSON 数组；失败返回 null。 */
    public static String recent(Context c, int limit) {
        if (!LOADED) return null;
        try {
            return nativeRecent(Config.get(c).toJson(), Config.dataDir(c), limit);
        } catch (Throwable t) {
            Log.e(TAG, "nativeRecent 失败", t);
            return null;
        }
    }

    /** AI 总结，返回 {"ok":...,"text":...,"error":...}；失败返回 null。 */
    public static String ai(Context c, String text) {
        if (!LOADED) return null;
        try {
            return nativeAi(Config.get(c).toJson(), Config.dataDir(c), text);
        } catch (Throwable t) {
            Log.e(TAG, "nativeAi 失败", t);
            return null;
        }
    }

    /** 推送，返回 {"ok":...,"detail":...}；失败返回 null。 */
    public static String push(Context c, String title, String content) {
        if (!LOADED) return null;
        try {
            return nativePush(Config.get(c).toJson(), Config.dataDir(c), title, content);
        } catch (Throwable t) {
            Log.e(TAG, "nativePush 失败", t);
            return null;
        }
    }

    public static boolean clear(Context c) {
        if (!LOADED) return false;
        try {
            return nativeClear(Config.get(c).toJson(), Config.dataDir(c)) == 1;
        } catch (Throwable t) {
            Log.e(TAG, "nativeClear 失败", t);
            return false;
        }
    }

    public static String selfTest(Context c) {
        if (!LOADED) return "Rust 核心未加载";
        try {
            return nativeSelfTest(Config.get(c).toJson());
        } catch (Throwable t) {
            return "自检失败：" + t.getMessage();
        }
    }

    // ---------------- 日程 / 待办 / 活动 ----------------

    /** 全部日程 JSON 数组；失败返回 null。 */
    public static String scheduleList(Context c) {
        if (!LOADED) return null;
        try {
            return nativeScheduleList(Config.get(c).toJson(), Config.dataDir(c));
        } catch (Throwable t) {
            Log.e(TAG, "nativeScheduleList 失败", t);
            return null;
        }
    }

    /** 保存一条日程，返回 {"ok":bool,...}。 */
    public static String scheduleSave(Context c, String itemJson) {
        if (!LOADED) return null;
        try {
            return nativeScheduleSave(Config.get(c).toJson(), Config.dataDir(c), itemJson);
        } catch (Throwable t) {
            Log.e(TAG, "nativeScheduleSave 失败", t);
            return null;
        }
    }

    public static boolean scheduleDelete(Context c, long id) {
        if (!LOADED) return false;
        try {
            return nativeScheduleDelete(Config.get(c).toJson(), Config.dataDir(c), id) == 1;
        } catch (Throwable t) {
            Log.e(TAG, "nativeScheduleDelete 失败", t);
            return false;
        }
    }

    public static boolean scheduleToggle(Context c, long id, boolean done) {
        if (!LOADED) return false;
        try {
            return nativeScheduleToggle(Config.get(c).toJson(), Config.dataDir(c), id, done) == 1;
        } catch (Throwable t) {
            Log.e(TAG, "nativeScheduleToggle 失败", t);
            return false;
        }
    }

    public static int scheduleClearDone(Context c) {
        if (!LOADED) return 0;
        try {
            return nativeScheduleClearDone(Config.get(c).toJson(), Config.dataDir(c));
        } catch (Throwable t) {
            Log.e(TAG, "nativeScheduleClearDone 失败", t);
            return 0;
        }
    }

    /** 判断一条通知/日程能否参加；返回 Assessment JSON，失败返回 null。 */
    public static String enrollCheck(Context c, String text) {
        if (!LOADED) return null;
        try {
            return nativeEnrollCheck(Config.get(c).toJson(), Config.dataDir(c),
                    text == null ? "" : text);
        } catch (Throwable t) {
            Log.e(TAG, "nativeEnrollCheck 失败", t);
            return null;
        }
    }

    /** 未来需要提醒的条目；触发时间由 Rust 计算。 */
    public static String scheduleUpcoming(Context c, long nowMs, long horizonMs) {
        if (!LOADED) return null;
        try {
            return nativeScheduleUpcoming(Config.get(c).toJson(), Config.dataDir(c),
                    nowMs, horizonMs);
        } catch (Throwable t) {
            Log.e(TAG, "nativeScheduleUpcoming 失败", t);
            return null;
        }
    }

    // ---------------- 课表 ----------------

    /** 课表视图 JSON。 */
    public static String timetable(Context c) {
        if (!LOADED) return null;
        try {
            return nativeTimetableGet(Config.get(c).toJson(), Config.dataDir(c));
        } catch (Throwable t) {
            Log.e(TAG, "nativeTimetableGet 失败", t);
            return null;
        }
    }

    /** 导入课表，返回 {"ok":bool,"count":n,"format":"..."}。 */
    public static String timetableImport(Context c, String text) {
        if (!LOADED) return null;
        try {
            return nativeTimetableImport(Config.get(c).toJson(), Config.dataDir(c),
                    text == null ? "" : text);
        } catch (Throwable t) {
            Log.e(TAG, "nativeTimetableImport 失败", t);
            return null;
        }
    }

    public static String timetableSetTerm(Context c, String termStart, int weeks) {
        if (!LOADED) return null;
        try {
            return nativeTimetableSetTerm(Config.get(c).toJson(), Config.dataDir(c),
                    termStart == null ? "" : termStart, weeks);
        } catch (Throwable t) {
            Log.e(TAG, "nativeTimetableSetTerm 失败", t);
            return null;
        }
    }

    public static String timetableDelete(Context c, int index) {
        if (!LOADED) return null;
        try {
            return nativeTimetableDelete(Config.get(c).toJson(), Config.dataDir(c), index);
        } catch (Throwable t) {
            Log.e(TAG, "nativeTimetableDelete 失败", t);
            return null;
        }
    }

    public static boolean timetableClear(Context c) {
        if (!LOADED) return false;
        try {
            return nativeTimetableClear(Config.get(c).toJson(), Config.dataDir(c)) == 1;
        } catch (Throwable t) {
            Log.e(TAG, "nativeTimetableClear 失败", t);
            return false;
        }
    }

    private static native String nativeVersion();

    private static native String nativeAnalyze(String cfgJson, String dataDir,
                                               String source, String text, boolean persist);

    private static native String nativeRecent(String cfgJson, String dataDir, int limit);

    private static native String nativeAi(String cfgJson, String dataDir, String text);

    private static native String nativePush(String cfgJson, String dataDir,
                                            String title, String content);

    private static native int nativeClear(String cfgJson, String dataDir);

    private static native String nativeSelfTest(String cfgJson);

    private static native String nativeScheduleList(String cfgJson, String dataDir);

    private static native String nativeScheduleSave(String cfgJson, String dataDir,
                                                    String itemJson);

    private static native int nativeScheduleDelete(String cfgJson, String dataDir, long id);

    private static native int nativeScheduleToggle(String cfgJson, String dataDir,
                                                   long id, boolean done);

    private static native int nativeScheduleClearDone(String cfgJson, String dataDir);

    private static native String nativeScheduleUpcoming(String cfgJson, String dataDir,
                                                        long nowMs, long horizonMs);

    private static native String nativeEnrollCheck(String cfgJson, String dataDir,
                                                   String text);

    private static native String nativeTimetableGet(String cfgJson, String dataDir);

    private static native String nativeTimetableImport(String cfgJson, String dataDir,
                                                       String text);

    private static native String nativeTimetableSetTerm(String cfgJson, String dataDir,
                                                        String termStart, int weeks);

    private static native String nativeTimetableDelete(String cfgJson, String dataDir,
                                                       int index);

    private static native int nativeTimetableClear(String cfgJson, String dataDir);
}
