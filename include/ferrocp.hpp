/**
 * @file ferrocp.hpp
 * @brief C++ wrapper for FerroCP - High-performance file copying library
 * 
 * This header provides a modern C++ interface for FerroCP, wrapping the
 * C API with RAII, exceptions, and STL containers.
 * 
 * @version 0.2.0
 * @author Long Hao <hal.long@outlook.com>
 * @copyright Apache-2.0 License
 */

#ifndef FERROCP_HPP
#define FERROCP_HPP

#include "ferrocp.h"
#include <string>
#include <vector>
#include <memory>
#include <functional>
#include <stdexcept>
#include <chrono>

namespace ferrocp {

/* ========================================================================== */
/* Exception Classes                                                         */
/* ========================================================================== */

/**
 * @brief Base exception class for FerroCP errors
 */
class Exception : public std::runtime_error {
public:
    explicit Exception(const std::string& message, int error_code = FERROCP_ERROR_GENERIC)
        : std::runtime_error(message), error_code_(error_code) {}
    
    int error_code() const noexcept { return error_code_; }

private:
    int error_code_;
};

/**
 * @brief Exception for file not found errors
 */
class FileNotFoundException : public Exception {
public:
    explicit FileNotFoundException(const std::string& path)
        : Exception("File not found: " + path, FERROCP_ERROR_FILE_NOT_FOUND) {}
};

/**
 * @brief Exception for permission denied errors
 */
class PermissionDeniedException : public Exception {
public:
    explicit PermissionDeniedException(const std::string& message)
        : Exception("Permission denied: " + message, FERROCP_ERROR_PERMISSION_DENIED) {}
};

/**
 * @brief Exception for insufficient space errors
 */
class InsufficientSpaceException : public Exception {
public:
    explicit InsufficientSpaceException(const std::string& message)
        : Exception("Insufficient space: " + message, FERROCP_ERROR_INSUFFICIENT_SPACE) {}
};

/* ========================================================================== */
/* Enums                                                                     */
/* ========================================================================== */

/**
 * @brief Copy operation modes
 */
enum class CopyMode {
    Copy = FERROCP_MODE_COPY,
    Move = FERROCP_MODE_MOVE,
    Sync = FERROCP_MODE_SYNC
};

/**
 * @brief Device types
 */
enum class DeviceType {
    Unknown = FERROCP_DEVICE_UNKNOWN,
    Hdd = FERROCP_DEVICE_HDD,
    Ssd = FERROCP_DEVICE_SSD,
    Network = FERROCP_DEVICE_NETWORK,
    RamDisk = FERROCP_DEVICE_RAMDISK
};

/**
 * @brief Performance rating
 */
enum class PerformanceRating {
    Poor = FERROCP_PERFORMANCE_POOR,
    Fair = FERROCP_PERFORMANCE_FAIR,
    Good = FERROCP_PERFORMANCE_GOOD,
    Excellent = FERROCP_PERFORMANCE_EXCELLENT
};

/* ========================================================================== */
/* Data Structures                                                          */
/* ========================================================================== */

/**
 * @brief Copy statistics
 */
struct CopyStats {
    uint64_t files_copied = 0;
    uint64_t directories_created = 0;
    uint64_t bytes_copied = 0;
    uint64_t files_skipped = 0;
    uint64_t errors = 0;
    std::chrono::milliseconds duration{0};
    double transfer_rate_mbps = 0.0;
    double efficiency_percent = 0.0;

    CopyStats() = default;
    explicit CopyStats(const ferrocp_stats_t& c_stats)
        : files_copied(c_stats.files_copied)
        , directories_created(c_stats.directories_created)
        , bytes_copied(c_stats.bytes_copied)
        , files_skipped(c_stats.files_skipped)
        , errors(c_stats.errors)
        , duration(c_stats.duration_ms)
        , transfer_rate_mbps(c_stats.transfer_rate_mbps)
        , efficiency_percent(c_stats.efficiency_percent) {}
};

/**
 * @brief Device information
 */
struct DeviceInfo {
    DeviceType device_type = DeviceType::Unknown;
    std::string filesystem;
    uint64_t total_space = 0;
    uint64_t available_space = 0;
    double read_speed_mbps = 0.0;
    double write_speed_mbps = 0.0;

    DeviceInfo() = default;
    explicit DeviceInfo(const ferrocp_device_info_t& c_info)
        : device_type(static_cast<DeviceType>(c_info.device_type ? std::stoi(c_info.device_type) : 0))
        , filesystem(c_info.filesystem ? c_info.filesystem : "")
        , total_space(c_info.total_space)
        , available_space(c_info.available_space)
        , read_speed_mbps(c_info.read_speed_mbps)
        , write_speed_mbps(c_info.write_speed_mbps) {}
};

/**
 * @brief Copy request parameters
 */
struct CopyRequest {
    std::string source;
    std::string destination;
    CopyMode mode = CopyMode::Copy;
    bool compress = false;
    bool preserve_metadata = true;
    bool verify_copy = false;
    unsigned int threads = 0;  // 0 = auto
    uint64_t buffer_size = 0;  // 0 = auto

    CopyRequest(const std::string& src, const std::string& dst)
        : source(src), destination(dst) {}
};

/* ========================================================================== */
/* Callback Types                                                           */
/* ========================================================================== */

/**
 * @brief Progress callback function type
 */
using ProgressCallback = std::function<void(
    double progress_percent,
    uint64_t bytes_copied,
    uint64_t total_bytes,
    const std::string& current_file
)>;

/**
 * @brief Error callback function type
 */
using ErrorCallback = std::function<void(
    int error_code,
    const std::string& error_message,
    const std::string& file_path
)>;

/* ========================================================================== */
/* Library Management                                                       */
/* ========================================================================== */

/**
 * @brief RAII wrapper for FerroCP library initialization
 */
class Library {
public:
    Library() {
        if (ferrocp_init() != 0) {
            throw Exception("Failed to initialize FerroCP library");
        }
    }

    ~Library() {
        ferrocp_cleanup();
    }

    // Non-copyable, non-movable
    Library(const Library&) = delete;
    Library& operator=(const Library&) = delete;
    Library(Library&&) = delete;
    Library& operator=(Library&&) = delete;

    std::string version() const {
        return ferrocp_version();
    }
};

/* ========================================================================== */
/* Engine Class                                                             */
/* ========================================================================== */

/**
 * @brief RAII wrapper for FerroCP engine
 */
class Engine {
public:
    Engine() : handle_(ferrocp_engine_create()) {
        if (handle_ == 0) {
            throw Exception("Failed to create FerroCP engine");
        }
    }

    ~Engine() {
        if (handle_ != 0) {
            ferrocp_engine_destroy(handle_);
        }
    }

    // Non-copyable, movable
    Engine(const Engine&) = delete;
    Engine& operator=(const Engine&) = delete;

    Engine(Engine&& other) noexcept : handle_(other.handle_) {
        other.handle_ = 0;
    }

    Engine& operator=(Engine&& other) noexcept {
        if (this != &other) {
            if (handle_ != 0) {
                ferrocp_engine_destroy(handle_);
            }
            handle_ = other.handle_;
            other.handle_ = 0;
        }
        return *this;
    }

    /**
     * @brief Execute a copy operation
     */
    CopyStats copy(const CopyRequest& request) {
        ferrocp_copy_request_t c_request = {
            request.source.c_str(),
            request.destination.c_str(),
            static_cast<int>(request.mode),
            request.compress ? 1 : 0,
            request.preserve_metadata ? 1 : 0,
            request.verify_copy ? 1 : 0,
            request.threads,
            request.buffer_size
        };

        ferrocp_result_t result = {};
        int error_code = ferrocp_copy(handle_, &c_request, &result);

        if (error_code != 0) {
            std::string message = result.error_message ? result.error_message : "Unknown error";
            ferrocp_free_result(&result);
            throw_exception_for_error_code(error_code, message);
        }

        ferrocp_free_result(&result);
        return CopyStats{}; // TODO: Extract stats from result
    }

    /**
     * @brief Execute a copy operation with progress callback
     */
    CopyStats copy_with_progress(
        const CopyRequest& request,
        const ProgressCallback& progress_callback = nullptr,
        const ErrorCallback& error_callback = nullptr
    ) {
        // TODO: Implement callback wrapper
        return copy(request);
    }

private:
    ferrocp_engine_handle_t handle_;

    void throw_exception_for_error_code(int error_code, const std::string& message) {
        switch (error_code) {
            case FERROCP_ERROR_FILE_NOT_FOUND:
                throw FileNotFoundException(message);
            case FERROCP_ERROR_PERMISSION_DENIED:
                throw PermissionDeniedException(message);
            case FERROCP_ERROR_INSUFFICIENT_SPACE:
                throw InsufficientSpaceException(message);
            default:
                throw Exception(message, error_code);
        }
    }
};

/* ========================================================================== */
/* Utility Functions                                                        */
/* ========================================================================== */

/**
 * @brief Get device information for a path
 */
inline DeviceInfo get_device_info(const std::string& path) {
    ferrocp_device_info_t c_info = {};
    int error_code = ferrocp_get_device_info(path.c_str(), &c_info);
    
    if (error_code != 0) {
        throw Exception("Failed to get device info for: " + path, error_code);
    }

    DeviceInfo info(c_info);
    ferrocp_free_device_info(&c_info);
    return info;
}

/**
 * @brief Check if a path exists
 */
inline bool path_exists(const std::string& path) {
    return ferrocp_path_exists(path.c_str()) != 0;
}

/**
 * @brief Check if a path is a directory
 */
inline bool is_directory(const std::string& path) {
    return ferrocp_is_directory(path.c_str()) != 0;
}

/**
 * @brief Check if a path is a file
 */
inline bool is_file(const std::string& path) {
    return ferrocp_is_file(path.c_str()) != 0;
}

/**
 * @brief Get file size
 */
inline uint64_t get_file_size(const std::string& path) {
    return ferrocp_get_file_size(path.c_str());
}

/**
 * @brief Join two paths
 */
inline std::string join_paths(const std::string& path1, const std::string& path2) {
    char* result = ferrocp_join_paths(path1.c_str(), path2.c_str());
    if (!result) {
        throw Exception("Failed to join paths: " + path1 + " and " + path2);
    }
    
    std::string joined(result);
    ferrocp_free_string(result);
    return joined;
}

/**
 * @brief Normalize a path
 */
inline std::string normalize_path(const std::string& path) {
    char* result = ferrocp_normalize_path(path.c_str());
    if (!result) {
        throw Exception("Failed to normalize path: " + path);
    }
    
    std::string normalized(result);
    ferrocp_free_string(result);
    return normalized;
}

} // namespace ferrocp

#endif /* FERROCP_HPP */
