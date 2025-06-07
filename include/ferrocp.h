/**
 * @file ferrocp.h
 * @brief C header file for FerroCP - High-performance file copying library
 * 
 * This header provides a C-compatible interface for FerroCP, enabling
 * integration with C++, Python, and other languages that can call C functions.
 * 
 * @version 0.2.0
 * @author Long Hao <hal.long@outlook.com>
 * @copyright Apache-2.0 License
 */

#ifndef FERROCP_H
#define FERROCP_H

#ifdef __cplusplus
extern "C" {
#endif

#include <stdint.h>
#include <stddef.h>

/* ========================================================================== */
/* Type Definitions                                                          */
/* ========================================================================== */

/**
 * @brief Copy operation modes
 */
typedef enum {
    FERROCP_MODE_COPY = 0,  /**< Copy files (default) */
    FERROCP_MODE_MOVE = 1,  /**< Move files (copy then delete source) */
    FERROCP_MODE_SYNC = 2   /**< Synchronize directories */
} ferrocp_copy_mode_t;

/**
 * @brief Device types
 */
typedef enum {
    FERROCP_DEVICE_UNKNOWN = 0,  /**< Unknown device type */
    FERROCP_DEVICE_HDD = 1,      /**< Hard Disk Drive */
    FERROCP_DEVICE_SSD = 2,      /**< Solid State Drive */
    FERROCP_DEVICE_NETWORK = 3,  /**< Network storage */
    FERROCP_DEVICE_RAMDISK = 4   /**< RAM disk */
} ferrocp_device_type_t;

/**
 * @brief Error codes
 */
typedef enum {
    FERROCP_SUCCESS = 0,              /**< Success */
    FERROCP_ERROR_GENERIC = 1,        /**< Generic error */
    FERROCP_ERROR_FILE_NOT_FOUND = 2, /**< File not found */
    FERROCP_ERROR_PERMISSION_DENIED = 3, /**< Permission denied */
    FERROCP_ERROR_INSUFFICIENT_SPACE = 4, /**< Insufficient space */
    FERROCP_ERROR_INVALID_PATH = 5,    /**< Invalid path */
    FERROCP_ERROR_NETWORK = 6,         /**< Network error */
    FERROCP_ERROR_COMPRESSION = 7,     /**< Compression error */
    FERROCP_ERROR_VERIFICATION = 8,    /**< Verification error */
    FERROCP_ERROR_CANCELLED = 9,       /**< Cancelled by user */
    FERROCP_ERROR_INVALID_ARGUMENT = 10, /**< Invalid argument */
    FERROCP_ERROR_OUT_OF_MEMORY = 11,  /**< Out of memory */
    FERROCP_ERROR_TIMEOUT = 12         /**< Timeout */
} ferrocp_error_code_t;

/**
 * @brief Performance rating
 */
typedef enum {
    FERROCP_PERFORMANCE_POOR = 0,      /**< Poor performance (< 25% efficiency) */
    FERROCP_PERFORMANCE_FAIR = 1,      /**< Fair performance (25-50% efficiency) */
    FERROCP_PERFORMANCE_GOOD = 2,      /**< Good performance (50-75% efficiency) */
    FERROCP_PERFORMANCE_EXCELLENT = 3  /**< Excellent performance (> 75% efficiency) */
} ferrocp_performance_rating_t;

/**
 * @brief Engine handle type
 */
typedef uint64_t ferrocp_engine_handle_t;

/**
 * @brief FFI-safe result structure
 */
typedef struct {
    int error_code;              /**< Error code (0 = success, non-zero = error) */
    const char* error_message;   /**< Error message (null if success) */
    const char* error_details;   /**< Additional error details as JSON string */
} ferrocp_result_t;

/**
 * @brief FFI-safe copy statistics
 */
typedef struct {
    uint64_t files_copied;           /**< Number of files copied */
    uint64_t directories_created;    /**< Number of directories created */
    uint64_t bytes_copied;           /**< Total bytes copied */
    uint64_t files_skipped;          /**< Number of files skipped */
    uint64_t errors;                 /**< Number of errors encountered */
    uint64_t duration_ms;            /**< Duration in milliseconds */
    double transfer_rate_mbps;       /**< Transfer rate in MB/s */
    double efficiency_percent;       /**< Performance efficiency percentage */
} ferrocp_stats_t;

/**
 * @brief FFI-safe device information
 */
typedef struct {
    const char* device_type;         /**< Device type as string */
    const char* filesystem;          /**< Filesystem type */
    uint64_t total_space;            /**< Total space in bytes */
    uint64_t available_space;        /**< Available space in bytes */
    double read_speed_mbps;          /**< Theoretical read speed in MB/s */
    double write_speed_mbps;         /**< Theoretical write speed in MB/s */
} ferrocp_device_info_t;

/**
 * @brief FFI-safe copy request
 */
typedef struct {
    const char* source;              /**< Source path */
    const char* destination;         /**< Destination path */
    int mode;                        /**< Copy mode (0=copy, 1=move, 2=sync) */
    int compress;                    /**< Enable compression */
    int preserve_metadata;           /**< Preserve metadata */
    int verify_copy;                 /**< Verify copy */
    unsigned int threads;            /**< Number of threads (0 = auto) */
    uint64_t buffer_size;            /**< Buffer size in bytes (0 = auto) */
} ferrocp_copy_request_t;

/**
 * @brief Progress callback function type
 * 
 * @param progress_percent Progress percentage (0.0 - 100.0)
 * @param bytes_copied Bytes copied so far
 * @param total_bytes Total bytes to copy
 * @param current_file Current file being processed (null-terminated string)
 * @param user_data User-provided data pointer
 */
typedef void (*ferrocp_progress_callback_t)(
    double progress_percent,
    uint64_t bytes_copied,
    uint64_t total_bytes,
    const char* current_file,
    void* user_data
);

/**
 * @brief Error callback function type
 * 
 * @param error_code Error code
 * @param error_message Error message (null-terminated string)
 * @param file_path File path where error occurred (null-terminated string)
 * @param user_data User-provided data pointer
 */
typedef void (*ferrocp_error_callback_t)(
    int error_code,
    const char* error_message,
    const char* file_path,
    void* user_data
);

/* ========================================================================== */
/* Library Management Functions                                              */
/* ========================================================================== */

/**
 * @brief Initialize FerroCP library
 * 
 * This function must be called before using any other FerroCP functions.
 * It initializes the async runtime and internal state.
 * 
 * @return 0 on success, non-zero error code on failure
 */
int ferrocp_init(void);

/**
 * @brief Cleanup FerroCP library
 * 
 * This function should be called when done using FerroCP to cleanup resources.
 */
void ferrocp_cleanup(void);

/**
 * @brief Get library version
 * 
 * @return Null-terminated string containing the library version
 *         The returned string is statically allocated and should not be freed
 */
const char* ferrocp_version(void);

/* ========================================================================== */
/* Engine Management Functions                                               */
/* ========================================================================== */

/**
 * @brief Create a new FerroCP engine
 * 
 * @return Handle to the engine, or 0 on failure
 */
ferrocp_engine_handle_t ferrocp_engine_create(void);

/**
 * @brief Destroy a FerroCP engine
 * 
 * Frees all resources associated with the engine handle.
 * 
 * @param handle Engine handle to destroy
 */
void ferrocp_engine_destroy(ferrocp_engine_handle_t handle);

/* ========================================================================== */
/* Copy Operations                                                           */
/* ========================================================================== */

/**
 * @brief Execute a copy operation
 * 
 * @param handle Engine handle
 * @param request Copy request parameters
 * @param result Result structure to fill (must be freed with ferrocp_free_result)
 * @return Error code (0 = success)
 */
int ferrocp_copy(
    ferrocp_engine_handle_t handle,
    const ferrocp_copy_request_t* request,
    ferrocp_result_t* result
);

/**
 * @brief Execute a copy operation with progress callback
 * 
 * @param handle Engine handle
 * @param request Copy request parameters
 * @param progress_callback Progress callback function (can be null)
 * @param error_callback Error callback function (can be null)
 * @param user_data User data passed to callbacks
 * @param result Result structure to fill (must be freed with ferrocp_free_result)
 * @return Error code (0 = success)
 */
int ferrocp_copy_with_progress(
    ferrocp_engine_handle_t handle,
    const ferrocp_copy_request_t* request,
    ferrocp_progress_callback_t progress_callback,
    ferrocp_error_callback_t error_callback,
    void* user_data,
    ferrocp_result_t* result
);

/* ========================================================================== */
/* Device Information Functions                                              */
/* ========================================================================== */

/**
 * @brief Get device information for a path
 * 
 * @param path Path to analyze (null-terminated string)
 * @param device_info Device information structure to fill
 * @return Error code (0 = success)
 */
int ferrocp_get_device_info(const char* path, ferrocp_device_info_t* device_info);

/**
 * @brief Free device information
 * 
 * @param device_info Device information structure to free
 */
void ferrocp_free_device_info(ferrocp_device_info_t* device_info);

/* ========================================================================== */
/* Memory Management Functions                                               */
/* ========================================================================== */

/**
 * @brief Free a string allocated by FerroCP
 * 
 * @param ptr String pointer to free
 */
void ferrocp_free_string(char* ptr);

/**
 * @brief Free a FerrocpResult structure
 * 
 * @param result Result structure to free
 */
void ferrocp_free_result(ferrocp_result_t* result);

/* ========================================================================== */
/* Error Handling Functions                                                  */
/* ========================================================================== */

/**
 * @brief Get error code description
 * 
 * @param error_code Error code
 * @return Static string describing the error (do not free)
 */
const char* ferrocp_error_code_description(int error_code);

/**
 * @brief Check if an error code represents success
 * 
 * @param error_code Error code to check
 * @return 1 if success, 0 if error
 */
int ferrocp_is_success(int error_code);

/**
 * @brief Check if an error code represents a recoverable error
 * 
 * @param error_code Error code to check
 * @return 1 if recoverable, 0 if not recoverable
 */
int ferrocp_is_recoverable_error(int error_code);

/* ========================================================================== */
/* Utility Functions                                                         */
/* ========================================================================== */

/**
 * @brief Get the size of a file in bytes
 * 
 * @param path File path (null-terminated string)
 * @return File size in bytes, or 0 if file doesn't exist or error
 */
uint64_t ferrocp_get_file_size(const char* path);

/**
 * @brief Check if a path exists
 * 
 * @param path Path to check (null-terminated string)
 * @return 1 if exists, 0 if not
 */
int ferrocp_path_exists(const char* path);

/**
 * @brief Check if a path is a directory
 * 
 * @param path Path to check (null-terminated string)
 * @return 1 if directory, 0 if not
 */
int ferrocp_is_directory(const char* path);

/**
 * @brief Check if a path is a file
 * 
 * @param path Path to check (null-terminated string)
 * @return 1 if file, 0 if not
 */
int ferrocp_is_file(const char* path);

/**
 * @brief Get the parent directory of a path
 * 
 * @param path Path (null-terminated string)
 * @return Parent path (must be freed with ferrocp_free_string), or null if no parent
 */
char* ferrocp_get_parent_path(const char* path);

/**
 * @brief Get the filename from a path
 * 
 * @param path Path (null-terminated string)
 * @return Filename (must be freed with ferrocp_free_string), or null if no filename
 */
char* ferrocp_get_filename(const char* path);

/**
 * @brief Join two paths
 * 
 * @param path1 First path (null-terminated string)
 * @param path2 Second path (null-terminated string)
 * @return Joined path (must be freed with ferrocp_free_string), or null on error
 */
char* ferrocp_join_paths(const char* path1, const char* path2);

/**
 * @brief Normalize a path (resolve . and .. components)
 * 
 * @param path Path to normalize (null-terminated string)
 * @return Normalized path (must be freed with ferrocp_free_string), or null on error
 */
char* ferrocp_normalize_path(const char* path);

#ifdef __cplusplus
}
#endif

#endif /* FERROCP_H */
