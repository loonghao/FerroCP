#!/usr/bin/env python3
"""
Basic test script for FerroCP Python bindings.
"""

import tempfile
import os
from pathlib import Path

def test_import():
    """Test basic import."""
    print("Testing import...")
    try:
        import ferrocp
        print(f"✓ Successfully imported ferrocp")
        print(f"✓ Version: {ferrocp.get_version()}")
        return True
    except Exception as e:
        print(f"✗ Import failed: {e}")
        return False

def test_basic_copy():
    """Test basic file copy functionality."""
    print("\nTesting basic copy...")
    try:
        import ferrocp
        
        # Create temporary files
        with tempfile.NamedTemporaryFile(mode='w', delete=False) as src:
            src.write("Hello, FerroCP!")
            src_path = src.name
        
        dst_path = src_path + ".copy"
        
        # Test quick copy
        print(f"Copying {src_path} to {dst_path}")
        result = ferrocp.quick_copy(src_path, dst_path)
        
        # Check if copy was successful
        if os.path.exists(dst_path):
            with open(dst_path, 'r') as f:
                content = f.read()
            if content == "Hello, FerroCP!":
                print("✓ Basic copy successful")
                success = True
            else:
                print(f"✗ Copy content mismatch: {content}")
                success = False
        else:
            print("✗ Destination file not created")
            success = False
        
        # Cleanup
        try:
            os.unlink(src_path)
            os.unlink(dst_path)
        except:
            pass
        
        return success
        
    except Exception as e:
        print(f"✗ Basic copy failed: {e}")
        import traceback
        traceback.print_exc()
        return False

def test_copy_options():
    """Test copy with options."""
    print("\nTesting copy with options...")
    try:
        import ferrocp
        
        # Create temporary files
        with tempfile.NamedTemporaryFile(mode='w', delete=False) as src:
            src.write("Test content for options")
            src_path = src.name
        
        dst_path = src_path + ".options"
        
        # Create copy options
        options = ferrocp.CopyOptions(
            verify=True,
            preserve_timestamps=True
        )
        
        print(f"Copying with options: verify={options.verify}, preserve_timestamps={options.preserve_timestamps}")
        
        # This might fail due to async nature, so we'll catch it
        try:
            result = ferrocp.copy_file(src_path, dst_path, options)
            print("✓ Copy with options initiated")
            success = True
        except Exception as e:
            print(f"✗ Copy with options failed: {e}")
            success = False
        
        # Cleanup
        try:
            os.unlink(src_path)
            if os.path.exists(dst_path):
                os.unlink(dst_path)
        except:
            pass
        
        return success
        
    except Exception as e:
        print(f"✗ Copy options test failed: {e}")
        import traceback
        traceback.print_exc()
        return False

def test_available_functions():
    """Test what functions are available."""
    print("\nTesting available functions...")
    try:
        import ferrocp
        
        print("Available functions and classes:")
        for name in sorted(dir(ferrocp)):
            if not name.startswith('_'):
                obj = getattr(ferrocp, name)
                obj_type = type(obj).__name__
                print(f"  {name}: {obj_type}")
        
        return True
        
    except Exception as e:
        print(f"✗ Function listing failed: {e}")
        return False

def main():
    """Run all tests."""
    print("FerroCP Python Bindings - Basic Tests")
    print("=" * 40)
    
    tests = [
        test_import,
        test_available_functions,
        test_basic_copy,
        test_copy_options,
    ]
    
    passed = 0
    total = len(tests)
    
    for test in tests:
        try:
            if test():
                passed += 1
        except Exception as e:
            print(f"✗ Test {test.__name__} crashed: {e}")
    
    print("\n" + "=" * 40)
    print(f"Tests passed: {passed}/{total}")
    
    if passed == total:
        print("🎉 All tests passed!")
        return 0
    else:
        print("❌ Some tests failed")
        return 1

if __name__ == "__main__":
    exit(main())
